//! HTML rendering with image inlining.

use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};

/// Render markdown to HTML, inlining images as data URLs.
pub(crate) fn render_html(md: &str, doc_base_dir: &Path) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(md, options);

    let events: Vec<Event<'_>> = parser
        .map(|event| match event {
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let new_url = inline_image(&dest_url, doc_base_dir);
                Event::Start(Tag::Image {
                    link_type,
                    dest_url: CowStr::from(new_url),
                    title,
                    id,
                })
            }
            other => other,
        })
        .collect();

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    html
}

/// Attempt to inline an image URL as a data URL.
/// Returns the original URL if the file doesn't exist or is not a relative path.
fn inline_image(url: &str, doc_base_dir: &Path) -> String {
    // Skip absolute URLs and data URLs
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("data:") {
        return url.to_string();
    }

    let image_path = doc_base_dir.join(url);
    if !image_path.exists() {
        eprintln!("Warning: image not found: {}", image_path.display());
        return url.to_string();
    }

    let data = match fs::read(&image_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "Warning: failed to read image {}: {}",
                image_path.display(),
                e
            );
            return url.to_string();
        }
    };

    if data.len() > 500_000 {
        eprintln!(
            "Warning: large image ({} KB): {}",
            data.len() / 1024,
            image_path.display()
        );
    }

    let mime = mime_from_extension(url);

    if mime == "image/svg+xml" {
        match String::from_utf8(data) {
            Ok(svg) => format!("data:{mime};utf8,{svg}"),
            Err(_) => url.to_string(),
        }
    } else {
        let encoded = BASE64.encode(&data);
        format!("data:{mime};base64,{encoded}")
    }
}

fn mime_from_extension(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn image_inlining() {
        let tmp = TempDir::new().unwrap();
        let img_path = tmp.path().join("test.png");
        // Minimal 1x1 PNG
        let png_data: [u8; 69] = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC,
            0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        fs::write(&img_path, &png_data).unwrap();

        let md = "![test](test.png)";
        let html = render_html(md, tmp.path());
        assert!(html.contains("data:image/png;base64,"));
        assert!(html.contains("alt=\"test\""));
    }

    #[test]
    fn image_missing_warns_keeps_url() {
        let html = render_html("![missing](nonexistent.png)", Path::new("/tmp/empty"));
        assert!(html.contains("nonexistent.png"));
    }

    #[test]
    fn image_absolute_url_unchanged() {
        let html = render_html("![pic](https://example.com/img.png)", Path::new("."));
        assert!(html.contains("https://example.com/img.png"));
    }
}
