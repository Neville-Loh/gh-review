/// Width in characters allocated to each line number column.
pub const LINE_NUM_WIDTH: usize = 5;

/// Total gutter width: two line-number columns plus separators (`old  new `).
pub const GUTTER_WIDTH: usize = LINE_NUM_WIDTH * 2 + 3;

/// Maximum outer width for a comment block (excluding gutter).
pub const COMMENT_BLOCK_MAX_WIDTH: u16 = 120;

/// Minimum gap between the block's right edge and the panel edge.
pub const COMMENT_BLOCK_RIGHT_MARGIN: u16 = 1;

/// Compute the effective inner content width for comment body text, given the
/// panel's wrap width. This accounts for gutter, block max width, right margin,
/// borders (2 chars), and horizontal padding (2 chars).
///
/// Both `model.rs` (pre-wrapping) and `comment_block.rs` (Block sizing) use
/// this to ensure the pre-wrapped line count matches the Block's inner height.
pub fn comment_body_width(wrap_width: usize) -> usize {
    let available = wrap_width.saturating_sub(GUTTER_WIDTH + COMMENT_BLOCK_RIGHT_MARGIN as usize);
    let block_width = available.min(COMMENT_BLOCK_MAX_WIDTH as usize);
    // 2 for left+right borders, 2 for horizontal padding (Padding::horizontal(1))
    block_width.saturating_sub(4)
}
