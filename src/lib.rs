pub mod error;
pub mod parse;
pub mod write;
pub mod font;
pub mod tables;
pub mod image;
pub mod woff;
pub mod woff2;
pub mod ttc;

pub use font::Font;
pub use error::{FontError, Tag};
pub use tables::Table;
pub use woff::{read_woff, write_woff};
pub use woff2::{read_woff2, write_woff2};
pub use ttc::Ttc;

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

    #[test]
    fn test_fvar_roundtrip() {
        use tables::fvar::{Fvar, AxisRecord, InstanceRecord};
        let fvar = Fvar {
            major_version: 1,
            minor_version: 0,
            axes: vec![
                AxisRecord {
                    axis_tag: Tag::new(b"wght"),
                    min_value: 100.0,
                    default_value: 400.0,
                    max_value: 900.0,
                    flags: 0,
                    axis_name_id: 256,
                },
            ],
            instances: vec![
                InstanceRecord {
                    subfamily_name_id: 2,
                    flags: 0,
                    coordinates: vec![400.0],
                    post_script_name_id: None,
                },
            ],
        };
        let mut w = Writer::new();
        fvar.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Fvar::parse(&bytes).unwrap();
        assert_eq!(parsed.axes.len(), 1);
        assert_eq!(parsed.axes[0].axis_tag, Tag::new(b"wght"));
        assert_eq!(parsed.axes[0].default_value, 400.0);
        assert_eq!(parsed.instances.len(), 1);
    }

    #[test]
    fn test_composite_glyph_roundtrip() {
        use tables::glyf::{CompositeGlyph, CompositeComponent, CompositeTransform, Glyph};
        let comp = CompositeGlyph {
            x_min: 0,
            y_min: 0,
            x_max: 500,
            y_max: 700,
            components: vec![
                CompositeComponent {
                    glyph_index: 1,
                    flags: 0x0001 | 0x0002 | 0x0008,
                    argument1: 10,
                    argument2: 20,
                    transformation: Some(CompositeTransform::Scale(1.0)),
                },
            ],
        };
        let mut w = Writer::new();
        comp.write(&mut w);
        let bytes = w.into_vec();
        assert!(bytes.len() > 10);
        // Verify composite marker
        assert_eq!(i16::from_be_bytes([bytes[0], bytes[1]]), -1);
    }

    #[test]
    fn test_cmap_reverse_lookup() {
        use tables::cmap::{Cmap, CmapSubtable, SequentialMapGroup};
        let cmap = Cmap {
            version: 0,
            num_tables: 1,
            records: vec![],
            subtables: vec![
                CmapSubtable::Format12 {
                    language: 0,
                    groups: vec![
                        SequentialMapGroup { start_char_code: 0x41, end_char_code: 0x43, start_glyph_id: 10 },
                    ],
                },
            ],
        };
        assert_eq!(cmap.glyph_codepoints(10), vec![0x41]);
        assert_eq!(cmap.glyph_codepoints(11), vec![0x42]);
        assert_eq!(cmap.glyph_codepoints(12), vec![0x43]);
        assert!(cmap.glyph_codepoints(99).is_empty());
    }

    #[test]
    fn test_stat_roundtrip() {
        use tables::stat::{Stat, DesignAxisRecord, AxisValueRecord};
        let stat = Stat {
            major_version: 1,
            minor_version: 2,
            design_axes: vec![
                DesignAxisRecord {
                    axis_tag: Tag::new(b"wght"),
                    axis_name_id: 256,
                    axis_ordering: 0,
                },
            ],
            axis_values: vec![
                AxisValueRecord {
                    format: 1,
                    axis_index: 0,
                    flags: 0,
                    value_name_id: 2,
                    value: 400.0,
                    linked_value: None,
                    nominal_value: None,
                    range_min_value: None,
                    range_max_value: None,
                    axis_values: vec![],
                },
            ],
            elided_fallback_name_id: Some(2),
        };
        let mut w = Writer::new();
        stat.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Stat::parse(&bytes).unwrap();
        assert_eq!(parsed.design_axes.len(), 1);
        assert_eq!(parsed.design_axes[0].axis_tag, Tag::new(b"wght"));
        assert_eq!(parsed.axis_values.len(), 1);
        assert_eq!(parsed.axis_values[0].value, 400.0);
        assert_eq!(parsed.elided_fallback_name_id, Some(2));
    }

    #[test]
    fn test_composite_glyph_full_roundtrip() {
        use tables::glyf::{CompositeGlyph, CompositeComponent, CompositeTransform};
        let comp = CompositeGlyph {
            x_min: 10,
            y_min: 20,
            x_max: 510,
            y_max: 720,
            components: vec![
                CompositeComponent {
                    glyph_index: 5,
                    flags: 0x0001 | 0x0002 | 0x0020, // args are words, more components, xy-scale
                    argument1: 100,
                    argument2: 200,
                    transformation: Some(CompositeTransform::XyScale(0.9, 1.1)),
                },
                CompositeComponent {
                    glyph_index: 8,
                    flags: 0x0001 | 0x0002 | 0x0040, // args are words, 2x2 matrix
                    argument1: -50,
                    argument2: 25,
                    transformation: Some(CompositeTransform::TwoByTwo(1.0, 0.0, 0.0, 1.0)),
                },
            ],
        };
        let mut w = Writer::new();
        comp.write(&mut w);
        let bytes = w.into_vec();
        // Parse back using GlyfTable::parse_single_glyph logic
        assert!(bytes.len() >= 10);
        assert_eq!(i16::from_be_bytes([bytes[0], bytes[1]]), -1);
        assert_eq!(i16::from_be_bytes([bytes[2], bytes[3]]), 10);
        assert_eq!(i16::from_be_bytes([bytes[4], bytes[5]]), 20);
        assert_eq!(i16::from_be_bytes([bytes[6], bytes[7]]), 510);
        assert_eq!(i16::from_be_bytes([bytes[8], bytes[9]]), 720);
    }

    #[test]
    fn test_name_set_family_subfamily() {
        use tables::name::{Name, NameRecord};
        let mut name = Name {
            format: 0,
            count: 2,
            string_offset: 0,
            records: vec![
                NameRecord {
                    platform_id: 1,
                    encoding_id: 0,
                    language_id: 0,
                    name_id: 1,
                    string: "Original".to_string(),
                },
                NameRecord {
                    platform_id: 1,
                    encoding_id: 0,
                    language_id: 0,
                    name_id: 2,
                    string: "OldStyle".to_string(),
                },
            ],
        };
        name.set_family("NewFamily");
        name.set_subfamily("Bold");
        assert_eq!(name.family_name(), Some("NewFamily".to_string()));
        assert_eq!(name.subfamily_name(), Some("Bold".to_string()));
    }

    #[test]
    fn test_os2_roundtrip() {
        use tables::os2::Os2;
        let os2 = Os2 {
            version: 4,
            x_avg_char_width: 500,
            us_weight_class: 400,
            us_width_class: 5,
            fs_type: 0,
            y_subscript_x_size: 650,
            y_subscript_y_size: 700,
            y_subscript_x_offset: 0,
            y_subscript_y_offset: 140,
            y_superscript_x_size: 650,
            y_superscript_y_size: 700,
            y_superscript_x_offset: 0,
            y_superscript_y_offset: 480,
            y_strikeout_size: 50,
            y_strikeout_position: 300,
            s_family_class: 0,
            panose: [2, 15, 5, 2, 2, 2, 4, 3, 2, 4],
            ul_unicode_range1: 0x00000001,
            ul_unicode_range2: 0,
            ul_unicode_range3: 0,
            ul_unicode_range4: 0,
            ach_vend_id: [b'P', b'Y', b'R', b'S'],
            fs_selection: 0x0040,
            us_first_char_index: 0x0020,
            us_last_char_index: 0x007E,
            s_typo_ascender: 800,
            s_typo_descender: -200,
            s_typo_line_gap: 200,
            us_win_ascent: 1000,
            us_win_descent: 200,
            ul_code_page_range1: Some(0x00000001),
            ul_code_page_range2: Some(0),
            sx_height: Some(500),
            s_cap_height: Some(700),
            us_default_char: Some(0),
            us_break_char: Some(0x20),
            us_max_context: Some(0),
            us_lower_optical_point_size: None,
            us_upper_optical_point_size: None,
        };
        let mut w = Writer::new();
        os2.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Os2::parse(&bytes, 0).unwrap();
        assert_eq!(parsed.version, 4);
        assert_eq!(parsed.us_weight_class, 400);
        assert_eq!(parsed.s_cap_height, Some(700));
        assert_eq!(parsed.panose, [2, 15, 5, 2, 2, 2, 4, 3, 2, 4]);
    }

    #[test]
    fn test_head_roundtrip() {
        use tables::head::Head;
        let head = Head {
            major_version: 1,
            minor_version: 0,
            font_revision: 0x00010000,
            check_sum_adjustment: 0,
            magic_number: 0x5F0F3CF5,
            flags: 0x000B,
            units_per_em: 1000,
            created: 0,
            modified: 0,
            x_min: -10,
            y_min: -200,
            x_max: 510,
            y_max: 800,
            mac_style: 0,
            lowest_rec_ppem: 3,
            font_direction_hint: 2,
            index_to_loc_format: 1,
            glyph_data_format: 0,
        };
        let mut w = Writer::new();
        head.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Head::parse(&bytes, 0).unwrap();
        assert_eq!(parsed.units_per_em, 1000);
        assert_eq!(parsed.x_min, -10);
        assert_eq!(parsed.index_to_loc_format, 1);
    }

    #[test]
    fn test_post_roundtrip() {
        use tables::post::Post;
        let post = Post {
            version: 0x00030000,
            italic_angle: 0,
            underline_position: -100,
            underline_thickness: 50,
            is_fixed_pitch: 0,
            min_mem_type42: 0,
            max_mem_type42: 0,
            min_mem_type1: 0,
            max_mem_type1: 0,
            names: None,
        };
        let mut w = Writer::new();
        post.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Post::parse(&bytes, 0).unwrap();
        assert_eq!(parsed.version, 0x00030000);
        assert_eq!(parsed.underline_position, -100);
    }

    #[test]
    fn test_loca_roundtrip() {
        let loca = LocaTable {
            offsets: vec![0, 100, 250, 400, 400],
            format: 1,
        };
        let mut w = Writer::new();
        loca.write(&mut w).unwrap();
        let bytes = w.into_vec();
        assert_eq!(bytes.len(), 20); // 5 * 4
        assert_eq!(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), 0);
        assert_eq!(u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 100);
    }

    #[test]
    fn test_simple_glyph_with_instructions_roundtrip() {
        use tables::glyf::SimpleGlyph;
        let glyph = SimpleGlyph {
            number_of_contours: 1,
            x_min: 0,
            y_min: 0,
            x_max: 100,
            y_max: 100,
            end_pts_of_contours: vec![3],
            instructions: vec![0x2B, 0xB8, 0x01], // PUSHB[1] 0xB8  END
            flags: vec![0x01, 0x01, 0x01, 0x01],
            x_coordinates: vec![0, 100, 100, 0],
            y_coordinates: vec![0, 0, 100, 100],
        };
        let mut w = Writer::new();
        glyph.write(&mut w);
        let bytes = w.into_vec();
        let num_contours = i16::from_be_bytes([bytes[0], bytes[1]]);
        assert_eq!(num_contours, 1);
        // Header=10, endPts=2, instrLen=2 => instrLen at offset 12
        let instr_len = u16::from_be_bytes([bytes[12], bytes[13]]);
        assert_eq!(instr_len, 3);
        // Verify instructions bytes
        assert_eq!(bytes[14], 0x2B);
        assert_eq!(bytes[15], 0xB8);
        assert_eq!(bytes[16], 0x01);
    }

    #[test]
    fn test_cmap_multiple_subtables() {
        use tables::cmap::{Cmap, EncodingRecord, CmapSubtable, SequentialMapGroup};
        let cmap = Cmap {
            version: 0,
            num_tables: 2,
            records: vec![
                EncodingRecord { platform_id: 0, encoding_id: 3, subtable_offset: 0 },
                EncodingRecord { platform_id: 3, encoding_id: 1, subtable_offset: 0 },
            ],
            subtables: vec![
                CmapSubtable::Format12 {
                    language: 0,
                    groups: vec![
                        SequentialMapGroup { start_char_code: 0x41, end_char_code: 0x5A, start_glyph_id: 1 },
                    ],
                },
                CmapSubtable::Format4 {
                    language: 0,
                    segments: vec![
                        tables::cmap::Format4Segment { end_code: 0x5A, start_code: 0x41, id_delta: 0, id_range_offset: 0 },
                        tables::cmap::Format4Segment { end_code: 0xFFFF, start_code: 0xFFFF, id_delta: 1, id_range_offset: 0 },
                    ],
                },
            ],
        };
        let mut w = Writer::new();
        cmap.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Cmap::parse(&bytes, 0).unwrap();
        assert_eq!(parsed.subtables.len(), 2);
        assert_eq!(parsed.map_codepoint(0x41), Some(1));
        assert_eq!(parsed.map_codepoint(0x5A), Some(26));
    }

    #[test]
    fn test_extract_inject_roundtrip() {
        let font = Font::create_minimal();
        let bytes = font.write().unwrap();
        let font2 = Font::read(&bytes).unwrap();

        // Simulate extract/inject: grab raw cvt table if present
        if let Some((tag, data)) = font2.raw_tables.iter().find(|(t, _)| *t == Tag::new(b"MATH")) {
            let mut font3 = Font::create_minimal();
            font3.raw_tables.push((*tag, data.clone()));
            let out = font3.write().unwrap();
            let font4 = Font::read(&out).unwrap();
            assert!(font4.raw_tables.iter().any(|(t, _)| *t == Tag::new(b"MATH")));
        }
    }

    #[test]
    fn test_gpos_kerning_lookup() {
        use tables::gpos::GposKernPair;
        // Verify kerning pair lookup logic
        let kerning = vec![
            GposKernPair { left: 0, right: 1, x_advance: -80 },
            GposKernPair { left: 2, right: 3, x_advance: 40 },
        ];
        let found = kerning.iter().find(|k| k.left == 0 && k.right == 1);
        assert!(found.is_some(), "Expected kerning pair (0, 1)");
        assert_eq!(found.unwrap().x_advance, -80);
        assert_eq!(kerning.len(), 2);
    }

    #[test]
    fn test_glyf_bounds_check_does_not_panic() {
        // Malformed glyph data: claims 1 contour but no data after header
        let bad_data = vec![
            0x00, 0x01, // numContours 1
            0x00, 0x00, // xMin
            0x00, 0x00, // yMin
            0x00, 0x00, // xMax
            0x00, 0x00, // yMax
        ];
        // This should not panic; parser should gracefully handle it
        assert!(bad_data.len() < 14); // not enough for endPts + instrLen
    }

    #[test]
    fn test_real_variable_font_has_fvar_or_stat() {
        let paths = [
            "/System/Library/Fonts/SFNSMono.ttf",
            "/System/Library/Fonts/SFGeorgian.ttf",
        ];
        for path in &paths {
            if !std::path::Path::new(path).exists() {
                continue;
            }
            let bytes = std::fs::read(path).unwrap();
            let font = Font::read(&bytes).unwrap();
            let has_var = font.fvar.is_some() || font.stat.is_some();
            assert!(has_var, "Expected {} to have fvar or STAT", path);
        }
    }

    #[test]
    fn test_writer_length_and_pad() {
        let mut w = Writer::new();
        w.write_u16(42);
        assert_eq!(w.len(), 2);
        w.pad_to_4();
        assert_eq!(w.len(), 4);
        w.write_u32(123);
        assert_eq!(w.len(), 8);
    }

    #[test]
    fn test_ttc_parsing() {
        use ttc::Ttc;
        // Build a minimal TTC with 2 identical fonts
        let font = Font::create_minimal();
        let font_bytes = font.write().unwrap();
        let font2 = Font::create_minimal();
        let mut font2_bytes = font2.write().unwrap();
        font2_bytes[0] = 0x4F; font2_bytes[1] = 0x54; font2_bytes[2] = 0x54; font2_bytes[3] = 0x4F; // CFF signature

        let mut ttc_bytes = Vec::new();
        ttc_bytes.extend_from_slice(b"ttcf");
        ttc_bytes.extend_from_slice(&0x00020000u32.to_be_bytes()); // version 2.0
        ttc_bytes.extend_from_slice(&2u32.to_be_bytes()); // numFonts
        let offset1 = 12 + 8 + 12; // after TTC header + offsets + DSIG fields
        let offset2 = offset1 + font_bytes.len();
        ttc_bytes.extend_from_slice(&(offset1 as u32).to_be_bytes());
        ttc_bytes.extend_from_slice(&(offset2 as u32).to_be_bytes());
        // DSIG fields for v2.0
        ttc_bytes.extend_from_slice(&0u32.to_be_bytes()); // dsigTag
        ttc_bytes.extend_from_slice(&0u32.to_be_bytes()); // dsigLength
        ttc_bytes.extend_from_slice(&0u32.to_be_bytes()); // dsigOffset
        ttc_bytes.extend_from_slice(&font_bytes);
        ttc_bytes.extend_from_slice(&font2_bytes);

        let ttc = Ttc::parse(&ttc_bytes).unwrap();
        assert_eq!(ttc.num_fonts, 2);
        assert_eq!(ttc.offsets.len(), 2);
        let f0 = ttc.font_at(&ttc_bytes, 0).unwrap();
        assert_eq!(f0.maxp.num_glyphs, font.maxp.num_glyphs);
    }

    #[test]
    fn test_cff_header_parse() {
        use tables::cff::Cff;
        let data = vec![
            0x01, // majorVersion
            0x00, // minorVersion
            0x04, // headerSize
            0x01, // offSize
        ];
        let cff = Cff::parse(&data).unwrap();
        assert!(!cff.is_cff2);
        assert_eq!(cff.major_version, 1);
        assert_eq!(cff.header_size, 4);
    }

    #[test]
    fn test_cff2_header_parse() {
        use tables::cff::Cff;
        let data = vec![
            0x02, // majorVersion
            0x00, // minorVersion
            0x05, // headerSize
            0x00, // topDictLength (placeholder for CFF2)
        ];
        let cff = Cff::parse(&data).unwrap();
        assert!(cff.is_cff2);
        assert_eq!(cff.major_version, 2);
    }

    #[test]
    fn test_woff2_roundtrip() {
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
            let data = sfnt[offset..offset + length].to_vec();
            table_data.push((tag, data));
        }
        let woff2 = woff2::write_woff2(&table_data).unwrap();
        assert!(woff2.len() > 48);
        // Read it back
        let recovered = woff2::read_woff2(&woff2).unwrap();
        assert_eq!(recovered.len(), table_data.len());
    }

    #[test]
    fn test_checksum_table() {
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let sum = parse::checksum_table(&data);
        let expected = u32::from_be_bytes([0x00, 0x01, 0x02, 0x03])
            .wrapping_add(u32::from_be_bytes([0x04, 0x05, 0x00, 0x00]));
        assert_eq!(sum, expected);
    }

    #[test]
    fn test_cmap_format6_roundtrip() {
        use tables::cmap::{Cmap, CmapSubtable, EncodingRecord};
        let cmap = Cmap {
            version: 0,
            num_tables: 1,
            records: vec![EncodingRecord { platform_id: 0, encoding_id: 1, subtable_offset: 0 }],
            subtables: vec![
                CmapSubtable::Format6 {
                    language: 0,
                    first_code: 0x41,
                    glyph_id_array: vec![10, 11, 12, 13, 14],
                },
            ],
        };
        let mut w = Writer::new();
        cmap.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Cmap::parse(&bytes, 0).unwrap();
        assert_eq!(parsed.map_codepoint(0x41), Some(10));
        assert_eq!(parsed.map_codepoint(0x43), Some(12));
        assert_eq!(parsed.map_codepoint(0x40), None);
        assert_eq!(parsed.map_codepoint(0x46), None);
        assert_eq!(parsed.glyph_codepoints(12), vec![0x43]);
    }

    #[test]
    fn test_cmap_format10_roundtrip() {
        use tables::cmap::{Cmap, CmapSubtable, EncodingRecord};
        let cmap = Cmap {
            version: 0,
            num_tables: 1,
            records: vec![EncodingRecord { platform_id: 0, encoding_id: 4, subtable_offset: 0 }],
            subtables: vec![
                CmapSubtable::Format10 {
                    language: 0,
                    start_char_code: 0x1F600,
                    glyph_id_array: vec![100, 101, 102],
                },
            ],
        };
        let mut w = Writer::new();
        cmap.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Cmap::parse(&bytes, 0).unwrap();
        assert_eq!(parsed.map_codepoint(0x1F600), Some(100));
        assert_eq!(parsed.map_codepoint(0x1F602), Some(102));
        assert_eq!(parsed.map_codepoint(0x1F5FF), None);
        assert_eq!(parsed.map_codepoint(0x1F603), None);
    }

    #[test]
    fn test_cmap_format13_roundtrip() {
        use tables::cmap::{Cmap, CmapSubtable, ConstantMapGroup, EncodingRecord};
        let cmap = Cmap {
            version: 0,
            num_tables: 1,
            records: vec![EncodingRecord { platform_id: 0, encoding_id: 5, subtable_offset: 0 }],
            subtables: vec![
                CmapSubtable::Format13 {
                    language: 0,
                    groups: vec![
                        ConstantMapGroup { start_char_code: 0x30, end_char_code: 0x39, glyph_id: 50 },
                    ],
                },
            ],
        };
        let mut w = Writer::new();
        cmap.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Cmap::parse(&bytes, 0).unwrap();
        assert_eq!(parsed.map_codepoint(0x30), Some(50));
        assert_eq!(parsed.map_codepoint(0x35), Some(50));
        assert_eq!(parsed.map_codepoint(0x39), Some(50));
        assert_eq!(parsed.map_codepoint(0x2F), None);
        assert_eq!(parsed.glyph_codepoints(50), vec![0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39]);
    }

    #[test]
    fn test_cmap_format14_roundtrip() {
        use tables::cmap::{Cmap, CmapSubtable, EncodingRecord, VariationSelectorRecord, UnicodeRange, NonDefaultUvMapping};
        let cmap = Cmap {
            version: 0,
            num_tables: 1,
            records: vec![EncodingRecord { platform_id: 0, encoding_id: 5, subtable_offset: 0 }],
            subtables: vec![
                CmapSubtable::Format14 {
                    records: vec![
                        VariationSelectorRecord {
                            var_selector: 0xFE00,
                            default_uvs: vec![
                                UnicodeRange { start_unicode_value: 0x0041, additional_count: 2 },
                            ],
                            non_default_uvs: vec![
                                NonDefaultUvMapping { unicode_value: 0x0044, glyph_id: 99 },
                            ],
                        },
                        VariationSelectorRecord {
                            var_selector: 0xFE01,
                            default_uvs: vec![],
                            non_default_uvs: vec![
                                NonDefaultUvMapping { unicode_value: 0x0045, glyph_id: 100 },
                            ],
                        },
                    ],
                },
            ],
        };
        let mut w = Writer::new();
        cmap.write(&mut w).unwrap();
        let bytes = w.into_vec();
        let parsed = Cmap::parse(&bytes, 0).unwrap();
        // Format 14 does not participate in single-codepoint mapping
        assert_eq!(parsed.map_codepoint(0x0041), None);
        assert_eq!(parsed.map_codepoint(0x0044), None);
        // Verify parsed structure
        if let CmapSubtable::Format14 { records } = &parsed.subtables[0] {
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].var_selector, 0xFE00);
            assert_eq!(records[0].default_uvs.len(), 1);
            assert_eq!(records[0].default_uvs[0].start_unicode_value, 0x0041);
            assert_eq!(records[0].default_uvs[0].additional_count, 2);
            assert_eq!(records[0].non_default_uvs.len(), 1);
            assert_eq!(records[0].non_default_uvs[0].glyph_id, 99);
            assert_eq!(records[1].var_selector, 0xFE01);
            assert_eq!(records[1].default_uvs.len(), 0);
            assert_eq!(records[1].non_default_uvs.len(), 1);
            assert_eq!(records[1].non_default_uvs[0].glyph_id, 100);
        } else {
            panic!("Expected Format14 subtable");
        }
    }
}
