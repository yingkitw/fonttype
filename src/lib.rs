pub mod error;
pub mod parse;
pub mod write;
pub mod font;
pub mod tables;
pub mod image;
pub mod woff;

pub use font::Font;
pub use error::{FontError, Tag};
pub use tables::Table;
pub use woff::{read_woff, write_woff};

#[cfg(test)]
mod tests {
    use super::*;
    use tables::glyf::Glyph;
    use tables::loca::LocaTable;
    use write::Writer;

    #[test]
    fn test_create_minimal_roundtrip() {
        let font = Font::create_minimal();
        let bytes = font.write().expect("write should succeed");
        let font2 = Font::read(&bytes).expect("read should succeed");

        assert_eq!(font.head.major_version, font2.head.major_version);
        assert_eq!(font.hhea, font2.hhea);
        assert_eq!(font.maxp, font2.maxp);
        assert_eq!(font.post, font2.post);
        assert_eq!(font.name.records.len(), font2.name.records.len());
        assert_eq!(font.cmap.subtables.len(), font2.cmap.subtables.len());
        assert_eq!(font.os2, font2.os2);
    }

    #[test]
    fn test_cmap_mapping() {
        let font = Font::create_minimal();
        assert_eq!(font.cmap.map_codepoint(0x20), Some(1));   // start of group
        assert_eq!(font.cmap.map_codepoint(0x7E), Some(95));  // 1 + (0x7E - 0x20)
        assert_eq!(font.cmap.map_codepoint(0x00), None);
    }

    #[test]
    fn test_name_lookup() {
        let font = Font::create_minimal();
        assert_eq!(font.name.family_name(), Some("TestFont".to_string()));
        assert_eq!(font.name.subfamily_name(), Some("Regular".to_string()));
    }

    #[test]
    fn test_glyf_roundtrip() {
        let mut font = Font::create_minimal();
        let rectangle = Glyph::from_points(vec![
            vec![(0, 0), (100, 0), (100, 100), (0, 100), (0, 0)],
        ]);
        if let Some(ref mut glyf) = font.glyf {
            glyf.glyphs.push(rectangle);
        }
        font.maxp.num_glyphs = 2;
        if let Some(ref mut loca) = font.loca {
            let sizes = if let Some(ref glyf) = font.glyf {
                glyf.glyphs.iter().map(|g| {
                    let mut w = write::Writer::new();
                    g.write(&mut w);
                    w.len()
                }).collect()
            } else { vec![] };
            *loca = LocaTable::from_glyph_sizes(&sizes, true);
        }

        let bytes = font.write().expect("write should succeed");
        let font2 = Font::read(&bytes).expect("read should succeed");

        assert!(font2.glyf.is_some());
        let glyf = font2.glyf.as_ref().unwrap();
        assert_eq!(glyf.glyphs.len(), 2);
        assert!(matches!(glyf.glyphs[1], Glyph::Simple(_)));
    }

    #[test]
    fn test_glyph_from_points() {
        let glyph = Glyph::from_points(vec![
            vec![(0, 0), (10, 0), (10, 10), (0, 10), (0, 0)],
        ]);
        if let Glyph::Simple(sg) = glyph {
            assert_eq!(sg.number_of_contours, 1);
            assert_eq!(sg.x_min, 0);
            assert_eq!(sg.y_min, 0);
            assert_eq!(sg.x_max, 10);
            assert_eq!(sg.y_max, 10);
            assert_eq!(sg.end_pts_of_contours, vec![4]);
        } else {
            panic!("Expected Simple glyph");
        }
    }

    #[test]
    fn test_image_tracer_rectangle() {
        let mut img = ::image::GrayImage::from_pixel(32, 32, ::image::Luma([255]));
        for y in 8..24 {
            for x in 8..24 {
                img.put_pixel(x, y, ::image::Luma([0]));
            }
        }
        let contours = image::tracer::trace_image(&img, 128);
        assert!(!contours.is_empty(), "tracer should find at least one contour");
        let first = &contours[0];
        assert!(first.len() >= 4, "contour should have at least 4 points");
    }

    #[test]
    fn test_rasterize_rectangle_glyph() {
        use tables::glyf::SimpleGlyph;
        let glyph = SimpleGlyph {
            number_of_contours: 1,
            x_min: 0,
            y_min: 0,
            x_max: 10,
            y_max: 10,
            end_pts_of_contours: vec![4],
            instructions: vec![],
            flags: vec![1, 1, 1, 1, 1],
            x_coordinates: vec![0, 10, 0, -10, 0],
            y_coordinates: vec![0, 0, 10, 0, -10],
        };
        let img = image::rasterizer::rasterize_glyph(&glyph, 64, 64);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 64);
        assert_eq!(img.get_pixel(32, 32)[0], 255);
    }

    #[test]
    fn test_hmtx_metric_lookup() {
        use tables::hmtx::{Hmtx, LongHorMetricRecord};
        let hmtx = Hmtx {
            h_metrics: vec![
                LongHorMetricRecord { advance_width: 500, lsb: 10 },
                LongHorMetricRecord { advance_width: 600, lsb: 20 },
            ],
            left_side_bearings: vec![30],
        };
        assert_eq!(hmtx.metric_for_glyph(0), (500, 10));
        assert_eq!(hmtx.metric_for_glyph(1), (600, 20));
        assert_eq!(hmtx.metric_for_glyph(2), (600, 30));
    }

    #[test]
    fn test_kern_lookup_and_roundtrip() {
        use tables::kern::{Kern, KernSubtable, KernPair};
        let kern = Kern {
            version: 0,
            n_tables: 1,
            subtables: vec![KernSubtable {
                version: 0,
                length: 0,
                coverage: 1,
                pairs: vec![
                    KernPair { left: 0, right: 1, value: -50 },
                    KernPair { left: 2, right: 3, value: 30 },
                ],
            }],
        };
        assert_eq!(kern.lookup(0, 1), Some(-50));
        assert_eq!(kern.lookup(2, 3), Some(30));
        assert_eq!(kern.lookup(0, 2), None);

        let mut w = write::Writer::new();
        kern.write(&mut w).unwrap();
        let written = w.into_vec();
        let parsed = Kern::parse(&written, 0).unwrap();
        assert_eq!(parsed.subtables[0].pairs.len(), 2);
        assert_eq!(parsed.subtables[0].pairs[0].value, -50);
    }

    #[test]
    fn test_subset_font() {
        let font = Font::create_minimal();
        let subset = font.subset(&[0]);
        assert_eq!(subset.maxp.num_glyphs, 1);
        assert_eq!(subset.hmtx.h_metrics.len(), 1);
        assert_eq!(subset.glyf.as_ref().unwrap().glyphs.len(), 1);
    }

    #[test]
    fn test_validate_minimal_font() {
        let font = Font::create_minimal();
        let bytes = font.write().unwrap();
        let font2 = Font::read(&bytes).unwrap();
        let issues = font2.validate(&bytes);
        assert!(!issues.iter().any(|i| i.contains("Missing required table")));
        assert!(!issues.iter().any(|i| i.contains("glyph count")));
    }

    #[test]
    fn test_read_real_font_geneva() {
        let path = "/System/Library/Fonts/Geneva.ttf";
        if !std::path::Path::new(path).exists() {
            return; // skip on systems without this font
        }
        let bytes = std::fs::read(path).unwrap();
        let font = Font::read(&bytes).unwrap();
        assert_eq!(font.name.family_name(), Some("Geneva".to_string()));
        assert!(font.maxp.num_glyphs > 0);
        assert!(font.head.units_per_em > 0);
        assert_eq!(font.hmtx.h_metrics.len() as u16, font.hhea.number_of_hmetrics);
    }

    #[test]
    fn test_gpos_parse_minimal() {
        use tables::gpos::Gpos;
        // Minimal GPOS header v1.0 with empty lookup list
        let data = vec![
            0x00, 0x01, // majorVersion 1
            0x00, 0x00, // minorVersion 0
            0x00, 0x0A, // scriptListOffset 10
            0x00, 0x0A, // featureListOffset 10
            0x00, 0x0A, // lookupListOffset 10
            0x00, 0x00, // lookupCount 0
        ];
        let gpos = Gpos::parse(&data).unwrap();
        assert!(gpos.kerning.is_empty());
    }

    #[test]
    fn test_gsub_parse_features() {
        use tables::gsub::Gsub;
        // GSUB header v1.0 with feature list at offset 10
        let mut data = vec![
            0x00, 0x01, // majorVersion 1
            0x00, 0x00, // minorVersion 0
            0x00, 0x0A, // scriptListOffset 10
            0x00, 0x0A, // featureListOffset 10
            0x00, 0x0A, // lookupListOffset 10
        ];
        // FeatureList at offset 10
        data.extend_from_slice(&[
            0x00, 0x02, // featureCount 2
            b'l', b'i', b'g', b'a', 0x00, 0x08, // featureRecord "liga" offset 8
            b'c', b'a', b'l', b't', 0x00, 0x10, // featureRecord "calt" offset 16
        ]);
        let gsub = Gsub::parse(&data).unwrap();
        assert_eq!(gsub.features, vec!["liga", "calt"]);
        assert!(gsub.has_ligatures());
    }

    #[test]
    fn test_validate_real_font_geneva() {
        let path = "/System/Library/Fonts/Geneva.ttf";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        let font = Font::read(&bytes).unwrap();
        let issues = font.validate(&bytes);
        assert!(!issues.iter().any(|i| i.contains("Missing required table")));
        assert!(!issues.iter().any(|i| i.contains("glyph count")));
        assert!(!issues.iter().any(|i| i.contains("hmtx entry count")));
        assert!(!issues.iter().any(|i| i.contains("extends beyond file")));
    }

    #[test]
    fn test_woff_roundtrip() {
        let font = Font::create_minimal();
        let sfnt = font.write().unwrap();

        // Parse sfnt to extract table data
        let mut p = parse::Parser::new(&sfnt, 0);
        let _sfnt_version = p.u32().unwrap();
        let num_tables = p.u16().unwrap();
        let _ = p.u16().unwrap();
        let _ = p.u16().unwrap();
        let _ = p.u16().unwrap();
        let mut table_data = Vec::with_capacity(num_tables as usize);
        for _ in 0..num_tables {
            let tag = p.tag().unwrap();
            let _checksum = p.u32().unwrap();
            let offset = p.u32().unwrap() as usize;
            let length = p.u32().unwrap() as usize;
            let data = p.buf()[offset..offset + length].to_vec();
            table_data.push((tag, data));
        }

        let woff = woff::write_woff(&table_data).unwrap();
        let recovered_sfnt = woff::read_woff(&woff).unwrap();
        let font2 = Font::read(&recovered_sfnt).unwrap();
        assert_eq!(font.name.family_name(), font2.name.family_name());
        assert_eq!(font.maxp.num_glyphs, font2.maxp.num_glyphs);
    }

    #[test]
    fn test_hvar_gvar_passthrough() {
        use tables::var::{Hvar, Gvar};
        let hvar = Hvar::parse(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(hvar.major_version, 1);
        let gvar = Gvar::parse(&[0x00, 0x01, 0x00, 0x00]).unwrap();
        assert_eq!(gvar.major_version, 1);

        let mut w = Writer::new();
        hvar.write(&mut w).unwrap();
        assert_eq!(w.into_vec().len(), 6);
    }
}
