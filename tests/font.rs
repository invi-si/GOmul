use wie_backend::canvas::{ArgbPixel, Canvas, Clip, Color, Font, Image, ImageBufferCanvas, TextAlignment, VecImageBuffer, string_width};

const FONT_DATA: &[u8] = include_bytes!("../assets/neodgm.ttf");
const WHITE: Color = Color {
    a: 0xff,
    r: 0xff,
    g: 0xff,
    b: 0xff,
};

fn raster_hash(font: &Font) -> u64 {
    let mut canvas = ImageBufferCanvas::new(VecImageBuffer::<ArgbPixel>::new(64, 32));
    canvas.draw_text(
        font,
        "A가",
        2,
        2,
        TextAlignment::Left,
        WHITE,
        Clip {
            x: 0,
            y: 0,
            width: 64,
            height: 32,
        },
    );
    let image = canvas.into_inner();
    let mut hash = 0xcbf29ce484222325u64;
    for y in 0..image.height() as i32 {
        for x in 0..image.width() as i32 {
            let color = image.get_pixel(x, y);
            for byte in [color.a, color.r, color.g, color.b] {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    hash
}

#[test]
fn injected_font_preserves_metrics_and_rasterization() {
    let static_font = Font::try_from_static(FONT_DATA).unwrap();
    let owned_font = Font::try_from_vec(FONT_DATA.to_vec()).unwrap();

    for font in [&static_font, &owned_font] {
        assert_eq!(string_width(font, "Hello, WIE!", 10.0), 73.333336);
        assert_eq!(string_width(font, "가나다", 10.0), 40.0);
        assert_eq!(raster_hash(font), 0x1026f6f7ab03dc46);
    }
}

#[test]
fn injected_font_respects_empty_clip() {
    let font = Font::try_from_static(FONT_DATA).unwrap();
    let mut canvas = ImageBufferCanvas::new(VecImageBuffer::<ArgbPixel>::new(30, 20));
    canvas.draw_text(
        &font,
        "A",
        2,
        2,
        TextAlignment::Left,
        WHITE,
        Clip {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
    );
    let image = canvas.into_inner();

    for y in 0..image.height() as i32 {
        for x in 0..image.width() as i32 {
            let color = image.get_pixel(x, y);
            assert_eq!([color.a, color.r, color.g, color.b], [0; 4]);
        }
    }
}

#[test]
fn invalid_font_data_is_rejected() {
    let error = Font::try_from_vec(vec![0, 1, 2, 3]).unwrap_err();
    assert!(error.to_string().contains("Invalid font data"));
}

#[test]
fn missing_direction_glyphs_have_distinct_readable_shapes_and_keep_spacing() {
    let font = Font::try_from_static(FONT_DATA).unwrap();
    let mut shapes = Vec::new();
    for arrow in ["←", "↑", "→", "↓"] {
        assert_eq!(string_width(&font, arrow, 10.0), string_width(&font, "\u{ffff}", 10.0));
        let mut canvas = ImageBufferCanvas::new(VecImageBuffer::<ArgbPixel>::new(24, 24));
        canvas.draw_text(
            &font,
            arrow,
            8,
            4,
            TextAlignment::Left,
            WHITE,
            Clip {
                x: 0,
                y: 0,
                width: 24,
                height: 24,
            },
        );
        let image = canvas.into_inner();
        let mut points = Vec::new();
        for y in 0..24 {
            for x in 0..24 {
                if image.get_pixel(x, y).a != 0 {
                    points.push((x, y));
                }
            }
        }
        assert!(!points.is_empty());
        let min_x = points.iter().map(|p| p.0).min().unwrap();
        let max_x = points.iter().map(|p| p.0).max().unwrap();
        let min_y = points.iter().map(|p| p.1).min().unwrap();
        let max_y = points.iter().map(|p| p.1).max().unwrap();
        assert_eq!((max_x - min_x, max_y - min_y), (6, 6));
        let tip_edge: Vec<_> = points
            .iter()
            .filter(|&&(x, y)| match arrow {
                "←" => x == min_x,
                "↑" => y == min_y,
                "→" => x == max_x,
                _ => y == max_y,
            })
            .collect();
        assert_eq!(tip_edge.len(), 1);
        assert!(!shapes.contains(&points));
        shapes.push(points);
    }
}

#[test]
fn direction_fallback_obeys_clip_and_alignment() {
    let font = Font::try_from_static(FONT_DATA).unwrap();
    let clip = Clip {
        x: 12,
        y: 8,
        width: 3,
        height: 3,
    };
    for alignment in [TextAlignment::Left, TextAlignment::Center, TextAlignment::Right] {
        let mut canvas = ImageBufferCanvas::new(VecImageBuffer::<ArgbPixel>::new(32, 24));
        canvas.draw_text(&font, "←↑→↓", 16, 4, alignment, WHITE, clip);
        let image = canvas.into_inner();
        for y in 0..24 {
            for x in 0..32 {
                if x < 12 || x >= 15 || y < 8 || y >= 11 {
                    assert_eq!(image.get_pixel(x, y).a, 0);
                }
            }
        }
    }
}
