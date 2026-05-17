/// Extract dominant color from an image (simplified approach)
pub async fn extract_dominant_color(image_source: &str) -> Option<String> {
    // For now, return a default vibrant color based on hash of the source
    // In production, you'd download and analyze the image
    
    let hash = image_source
        .chars()
        .map(|c| c as u32)
        .sum::<u32>();
    
    // Generate a consistent color from the hash
    let r = ((hash >> 16) & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = (hash & 0xFF) as u8;
    
    Some(format!("#{:02x}{:02x}{:02x}", r, g, b))
}
