//! A simple texture atlas for online generation.
use cgmath::Vector2;
use image::{imageops, GenericImage, RgbaImage};

use crate::container::SimpleSlab;
use crate::create_simple_key;

const INITIAL_HEIGHT: u32 = 2048;
const GROWTH_HEIGHT: u32 = 2048;

/// Represents a rectangle in 2D space.
#[derive(Copy, Clone, Debug)]
pub struct Rectangle {
    /// The minimal point of the rectangle (should be top left).
    pub min: Vector2<u32>,
    /// The maximal point of the rectangle (should be bottom right).
    pub max: Vector2<u32>,
}

impl Rectangle {
    /// Creates a new [`Rectangle`] with given minimum and maximum coordinates.
    pub fn new(min: Vector2<u32>, max: Vector2<u32>) -> Self {
        Self { min, max }
    }

    /// Returns the height of the rectangle.
    pub fn height(&self) -> u32 {
        self.max.y - self.min.y
    }

    /// Returns the width of the rectangle.
    pub fn width(&self) -> u32 {
        self.max.x - self.min.x
    }

    /// Checks if this rectangle can fit another rectangle of given size.
    fn can_fit(&self, size: Vector2<u32>) -> bool {
        self.width() >= size.x && self.height() >= size.y
    }
}

impl PartialEq for Rectangle {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

/// Represents an allocated rectangle in the texture atlas.
pub struct Allocation {
    /// The rectangle that was allocated.
    pub rectangle: Rectangle,
}

impl Allocation {
    /// Maps normalized coordinates to atlas coordinates.
    pub fn map_to_atlas(&self, norm_x: f32, norm_y: f32) -> (u32, u32) {
        let atlas_width = self.rectangle.width();
        let atlas_height = self.rectangle.height();

        let x = (norm_x * atlas_width as f32).round() as u32 + self.rectangle.min.x;
        let y = (norm_y * atlas_height as f32).round() as u32 + self.rectangle.min.y;

        (x.min(self.rectangle.max.x), y.min(self.rectangle.max.y))
    }
}

/// A texture atlas implementation using the MAXRECTS-BSSF (Best Short Side Fit)
/// algorithm.
///
/// This implementation is based on the algorithm described in the paper:
/// "A Thousand Ways to Pack the Bin - A Practical Approach to Two-Dimensional
/// Rectangle Bin Packing" by Jukka Jylänki (2010).
///
/// Key features of this implementation:
/// - Uses the Best Short Side Fit (BSSF) heuristic for rectangle placement.
/// - Maintains a list of free rectangles, splitting them as allocations are
///   made.
/// - Implements a method to remove redundant (contained) free rectangles for
///   efficiency.
/// - Supports dynamic growth of the atlas up to a specified maximum size.
///
/// The time complexity of this algorithm is O(n * m^2), where n is the number
/// of rectangles to be packed and m is the number of free rectangles at any
/// given time. While this can be relatively slow for large numbers of
/// rectangles, it generally produces very efficient packings.
pub struct TextureAtlas {
    size: Vector2<u32>,
    max_size: Vector2<u32>,
    free_rects: Vec<Rectangle>,
    image: RgbaImage,
}

impl TextureAtlas {
    /// Creates a new [`TextureAtlas`]. It will start with the given maximum
    /// width and `INITIAL_HEIGHT`. It will then grow with `GROW_HEIGHT` on
    /// demand.
    pub fn new(max_size: Vector2<u32>) -> Self {
        let initial_size = Vector2::new(max_size.x, INITIAL_HEIGHT);
        TextureAtlas {
            size: initial_size,
            max_size,
            free_rects: vec![Rectangle::new(Vector2::new(0, 0), initial_size)],
            image: RgbaImage::new(initial_size.x, initial_size.y),
        }
    }

    /// Allocates space in the atlas for a rectangle of the given size.
    /// Grows the atlas if necessary, up to the maximum size.
    ///
    /// Returns `Some(Allocation)` if successful, `None` if allocation fails.
    pub fn allocate(&mut self, size: Vector2<u32>) -> Option<Allocation> {
        if size.x > self.max_size.x || size.y > self.max_size.y {
            return None;
        }

        // Try to allocate with current size.
        let allocation = self.try_allocate(size);
        if allocation.is_some() {
            return allocation;
        }

        // If allocation failed, try to grow the atlas.
        while self.size.y < self.max_size.y {
            self.grow();
            let allocation = self.try_allocate(size);
            if allocation.is_some() {
                return allocation;
            }
        }

        // Allocation failed even after growing to max size.
        None
    }

    fn try_allocate(&mut self, size: Vector2<u32>) -> Option<Allocation> {
        let best_rect_index = self.find_best_rect(size)?;
        let free_rect = self.free_rects.remove(best_rect_index);

        let allocation = Allocation {
            rectangle: Rectangle::new(free_rect.min, free_rect.min + size),
        };

        self.split_free_rect(&free_rect, &allocation.rectangle);

        Some(allocation)
    }

    /// Allocates space for the given image and writes its data to the atlas.
    ///
    /// Returns `Some(Allocation)` if successful, `None` if allocation fails.
    pub fn allocate_with_data(&mut self, image: &RgbaImage) -> Option<Allocation> {
        let size = Vector2::new(image.width(), image.height());
        let allocation = self.allocate(size)?;
        self.write_image_data(&allocation, image);
        Some(allocation)
    }

    /// Saves the atlas image at the given path.
    pub fn save_atlas(&self, path: &str) -> Result<(), image::ImageError> {
        self.image.save(path)
    }

    /// Implements the BSSF (Best Short Side Fit) heuristics.
    fn find_best_rect(&self, size: Vector2<u32>) -> Option<usize> {
        self.free_rects
            .iter()
            .enumerate()
            .filter(|(_, rectangle)| rectangle.can_fit(size))
            .min_by_key(|(_, rectangle)| {
                let leftover_horizontal = rectangle.width().saturating_sub(size.x);
                let leftover_vertical = rectangle.height().saturating_sub(size.y);
                std::cmp::min(leftover_horizontal, leftover_vertical)
            })
            .map(|(index, _)| index)
    }

    fn split_free_rect(&mut self, free_rect: &Rectangle, used_rect: &Rectangle) {
        // Right split.
        if used_rect.max.x < free_rect.max.x {
            self.free_rects
                .push(Rectangle::new(Vector2::new(used_rect.max.x, free_rect.min.y), free_rect.max));
        }

        // Bottom split.
        if used_rect.max.y < free_rect.max.y {
            self.free_rects.push(Rectangle::new(
                Vector2::new(free_rect.min.x, used_rect.max.y),
                Vector2::new(used_rect.max.x, free_rect.max.y),
            ));
        }
    }

    fn grow(&mut self) {
        let old_height = self.size.y;
        self.size.y = (self.size.y + GROWTH_HEIGHT).min(self.max_size.y);

        // Add new free rectangle for the grown area.
        self.free_rects.push(Rectangle::new(
            Vector2::new(0, old_height),
            Vector2::new(self.size.x, self.size.y),
        ));

        let mut new_image = RgbaImage::new(self.size.x, self.size.y);
        let _ = new_image.copy_from(&self.image, 0, 0);
        self.image = new_image;
    }

    fn write_image_data(&mut self, allocation: &Allocation, image: &RgbaImage) {
        imageops::replace(
            &mut self.image,
            image,
            allocation.rectangle.min.x as _,
            allocation.rectangle.min.y as _,
        );
    }
}

create_simple_key!(NodeKey);

struct Node {
    children: Option<[NodeKey; 2]>,
    rectangle: Rectangle,
    is_filled: bool,
}

impl Node {
    fn new(rectangle: Rectangle) -> Self {
        Self {
            children: None,
            rectangle,
            is_filled: false,
        }
    }
}

pub struct BinaryTreeTextureAtlas {
    size: Vector2<u32>,
    max_size: Vector2<u32>,
    nodes: SimpleSlab<NodeKey, Node>,
    root: NodeKey,
    image: RgbaImage,
}

impl BinaryTreeTextureAtlas {
    pub fn new(max_size: Vector2<u32>) -> Self {
        let initial_size = Vector2::new(max_size.x, INITIAL_HEIGHT);
        let mut nodes = SimpleSlab::with_capacity(64);
        let root = nodes.insert(Node::new(Rectangle::new(Vector2::new(0, 0), initial_size))).unwrap();

        BinaryTreeTextureAtlas {
            size: initial_size,
            max_size,
            nodes,
            root,
            image: RgbaImage::new(initial_size.x, initial_size.y),
        }
    }

    pub fn allocate_with_data(&mut self, image: &RgbaImage) -> Option<Allocation> {
        let size = Vector2::new(image.width(), image.height());
        let allocation = self.allocate(size)?;
        self.write_image_data(&allocation, image);
        Some(allocation)
    }

    pub fn allocate(&mut self, size: Vector2<u32>) -> Option<Allocation> {
        if size.x > self.max_size.x || size.y > self.max_size.y {
            return None;
        }

        let allocation = self.try_allocate(size);
        if allocation.is_some() {
            return allocation;
        }

        while self.size.y < self.max_size.y {
            self.grow();
            let allocation = self.try_allocate(size);
            if allocation.is_some() {
                return allocation;
            }
        }

        None
    }

    fn try_allocate(&mut self, size: Vector2<u32>) -> Option<Allocation> {
        self.insert(self.root, size).map(|rectangle| Allocation { rectangle })
    }

    fn insert(&mut self, node_key: NodeKey, size: Vector2<u32>) -> Option<Rectangle> {
        if let Some(children) = self.nodes.get(node_key).unwrap().children {
            return self.insert(children[0], size).or_else(|| self.insert(children[1], size));
        }

        let node = &mut self.nodes.get_mut(node_key).unwrap();

        if node.is_filled {
            return None;
        }

        if size.x > node.rectangle.width() || size.y > node.rectangle.height() {
            return None;
        }

        if size.x == node.rectangle.width() && size.y == node.rectangle.height() {
            node.is_filled = true;
            return Some(node.rectangle);
        }

        let dw = node.rectangle.width() - size.x;
        let dh = node.rectangle.height() - size.y;

        let (child1_rect, child2_rect) = if dw > dh {
            (
                Rectangle::new(
                    node.rectangle.min,
                    Vector2::new(node.rectangle.min.x + size.x, node.rectangle.max.y),
                ),
                Rectangle::new(
                    Vector2::new(node.rectangle.min.x + size.x, node.rectangle.min.y),
                    node.rectangle.max,
                ),
            )
        } else {
            (
                Rectangle::new(
                    node.rectangle.min,
                    Vector2::new(node.rectangle.max.x, node.rectangle.min.y + size.y),
                ),
                Rectangle::new(
                    Vector2::new(node.rectangle.min.x, node.rectangle.min.y + size.y),
                    node.rectangle.max,
                ),
            )
        };

        let child1 = self.nodes.insert(Node::new(child1_rect)).unwrap();
        let child2 = self.nodes.insert(Node::new(child2_rect)).unwrap();

        self.nodes.get_mut(node_key).unwrap().children = Some([child1, child2]);

        self.insert(child1, size)
    }

    pub fn save_atlas(&self, path: &str) -> Result<(), image::ImageError> {
        self.image.save(path)
    }

    fn grow(&mut self) {
        let old_height = self.size.y;
        self.size.y = (self.size.y + GROWTH_HEIGHT).min(self.max_size.y);

        let new_area = Rectangle::new(Vector2::new(0, old_height), Vector2::new(self.size.x, self.size.y));
        let new_root = self.nodes.insert(Node::new(Rectangle::new(Vector2::new(0, 0), self.size))).unwrap();
        let new_area_node = self.nodes.insert(Node::new(new_area)).unwrap();
        self.nodes.get_mut(new_root).unwrap().children = Some([self.root, new_area_node]);
        self.root = new_root;

        let mut new_image = RgbaImage::new(self.size.x, self.size.y);
        let _ = new_image.copy_from(&self.image, 0, 0);
        self.image = new_image;
    }

    fn write_image_data(&mut self, allocation: &Allocation, image: &RgbaImage) {
        imageops::replace(
            &mut self.image,
            image,
            allocation.rectangle.min.x as _,
            allocation.rectangle.min.y as _,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangles_overlap(rect1: &Rectangle, rect2: &Rectangle) -> bool {
        rect1.min.x < rect2.max.x && rect1.max.x > rect2.min.x && rect1.min.y < rect2.max.y && rect1.max.y > rect2.min.y
    }

    #[test]
    fn test_allocate_single_rectangle() {
        let mut atlas = TextureAtlas::new(Vector2::new(1024, 1024));
        let allocation = atlas.allocate(Vector2::new(100, 100));
        assert!(allocation.is_some());
        let alloc = allocation.unwrap();
        assert_eq!(alloc.rectangle.min, Vector2::new(0, 0));
        assert_eq!(alloc.rectangle.max, Vector2::new(100, 100));
    }

    #[test]
    fn test_allocate_fail() {
        let mut atlas = TextureAtlas::new(Vector2::new(100, 100));
        assert!(atlas.allocate(Vector2::new(101, 50)).is_none());
        assert!(atlas.allocate(Vector2::new(50, 101)).is_none());
    }

    #[test]
    fn test_map_to_atlas() {
        let mut atlas = TextureAtlas::new(Vector2::new(1024, 1024));
        let alloc = atlas.allocate(Vector2::new(100, 200)).unwrap();

        assert_eq!(alloc.map_to_atlas(0.0, 0.0), (0, 0));
        assert_eq!(alloc.map_to_atlas(1.0, 1.0), (100, 200));
        assert_eq!(alloc.map_to_atlas(0.5, 0.5), (50, 100));
    }

    #[test]
    fn test_multiple_allocations() {
        let mut atlas = TextureAtlas::new(Vector2::new(1024, 1024));
        let alloc1 = atlas.allocate(Vector2::new(100, 100)).unwrap();
        let alloc2 = atlas.allocate(Vector2::new(200, 200)).unwrap();
        let alloc3 = atlas.allocate(Vector2::new(300, 300)).unwrap();

        assert_eq!(alloc1.rectangle.min, Vector2::new(0, 0));
        assert_eq!(alloc1.rectangle.max, Vector2::new(100, 100));

        assert!(alloc2.rectangle.min.x >= 100 || alloc2.rectangle.min.y >= 100);
        assert_eq!(alloc2.rectangle.width(), 200);
        assert_eq!(alloc2.rectangle.height(), 200);

        assert!(alloc3.rectangle.min.x >= 200 || alloc3.rectangle.min.y >= 200);
        assert_eq!(alloc3.rectangle.width(), 300);
        assert_eq!(alloc3.rectangle.height(), 300);
    }

    #[test]
    fn test_allocation_edge_cases() {
        let mut atlas = TextureAtlas::new(Vector2::new(1024, INITIAL_HEIGHT));

        // Test allocating the entire atlas.
        let full_alloc = atlas.allocate(Vector2::new(1024, INITIAL_HEIGHT));
        assert!(full_alloc.is_some());

        // Test allocating after full allocation.
        let fail_alloc = atlas.allocate(Vector2::new(1, 1));
        assert!(fail_alloc.is_none());
    }

    #[test]
    fn test_atlas_growth() {
        let mut atlas = TextureAtlas::new(Vector2::new(1024, 4096));
        let initial_height = atlas.size.y;

        // Allocate until we force growth.
        let mut allocations = Vec::new();
        while atlas.size.y == initial_height {
            if let Some(alloc) = atlas.allocate(Vector2::new(512, 512)) {
                allocations.push(alloc);
            } else {
                break;
            }
        }

        assert!(atlas.size.y > initial_height);
        assert!(atlas.size.y <= 4096);
    }

    #[test]
    fn test_no_rectangle_overlap() {
        let mut atlas = TextureAtlas::new(Vector2::new(1024, 1024));
        let mut allocations = Vec::new();

        for _ in 0..10 {
            if let Some(alloc) = atlas.allocate(Vector2::new(100, 100)) {
                allocations.push(alloc);
            }
        }

        for (i, alloc1) in allocations.iter().enumerate() {
            for (j, alloc2) in allocations.iter().enumerate() {
                if i != j {
                    assert!(
                        !rectangles_overlap(&alloc1.rectangle, &alloc2.rectangle),
                        "Overlap detected between rectangle {} and rectangle {}",
                        i,
                        j
                    );
                }
            }
        }
    }

    #[test]
    fn test_no_rectangle_overlap_varied_sizes() {
        let mut atlas = TextureAtlas::new(Vector2::new(1024, 1024));
        let mut allocations = Vec::new();

        let sizes = [
            Vector2::new(50, 50),
            Vector2::new(200, 200),
            Vector2::new(100, 100),
            Vector2::new(300, 100),
            Vector2::new(100, 300),
            Vector2::new(25, 25),
            Vector2::new(400, 400),
        ];

        for size in sizes.iter() {
            if let Some(alloc) = atlas.allocate(*size) {
                allocations.push(alloc);
            }
        }

        for _ in 0..20 {
            if let Some(alloc) = atlas.allocate(Vector2::new(10, 10)) {
                allocations.push(alloc);
            }
        }

        for (i, alloc1) in allocations.iter().enumerate() {
            for (j, alloc2) in allocations.iter().enumerate() {
                if i != j {
                    assert!(
                        !rectangles_overlap(&alloc1.rectangle, &alloc2.rectangle),
                        "Overlap detected between rectangle {} ({:?}) and rectangle {} ({:?})",
                        i,
                        alloc1.rectangle,
                        j,
                        alloc2.rectangle
                    );
                }
            }
        }
    }

    #[test]
    fn test_no_rectangle_overlap_after_growth() {
        let mut atlas = TextureAtlas::new(Vector2::new(1024, 4096));
        let initial_height = atlas.size.y;

        let mut allocations = Vec::new();
        while atlas.size.y == initial_height {
            if let Some(alloc) = atlas.allocate(Vector2::new(512, 512)) {
                allocations.push(alloc);
            } else {
                break;
            }
        }

        assert!(atlas.size.y > initial_height);

        for _ in 0..10 {
            if let Some(alloc) = atlas.allocate(Vector2::new(50, 50)) {
                allocations.push(alloc);
            }
        }

        for (i, alloc1) in allocations.iter().enumerate() {
            for (j, alloc2) in allocations.iter().enumerate() {
                if i != j {
                    assert!(
                        !rectangles_overlap(&alloc1.rectangle, &alloc2.rectangle),
                        "Overlap detected between rectangle {} ({:?}) and rectangle {} ({:?})",
                        i,
                        alloc1.rectangle,
                        j,
                        alloc2.rectangle
                    );
                }
            }
        }
    }
}
