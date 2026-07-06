use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::{Table, head::Head, hhea::Hhea, maxp::Maxp, post::Post, name::Name, cmap::Cmap, os2::Os2, glyf::GlyfTable, loca::LocaTable, hmtx::Hmtx, kern::Kern, gpos::Gpos, gsub::Gsub, var::{Hvar, Gvar}, fvar::Fvar, stat::Stat, cff::Cff, colr::Colr, cpal::Cpal, svg::Svg};
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub enum SfntVersion {
    TrueType,
    Cff,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableRecord {
    pub tag: Tag,
    pub checksum: u32,
    pub offset: u32,
    pub length: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    pub sfnt_version: SfntVersion,
    pub tables: Vec<TableRecord>,
    pub head: Head,
    pub hhea: Hhea,
    pub maxp: Maxp,
    pub post: Post,
    pub name: Name,
    pub cmap: Cmap,
    pub os2: Os2,
    pub glyf: Option<GlyfTable>,
    pub loca: Option<LocaTable>,
    pub hmtx: Hmtx,
    pub kern: Option<Kern>,
    pub cvt: Option<Vec<u8>>,
    pub prep: Option<Vec<u8>>,
    pub fpgm: Option<Vec<u8>>,
    pub gpos: Option<Gpos>,
    pub gsub: Option<Gsub>,
    pub hvar: Option<Hvar>,
    pub gvar: Option<Gvar>,
    pub fvar: Option<Fvar>,
    pub stat: Option<Stat>,
    pub cff: Option<Cff>,
    pub colr: Option<Colr>,
    pub cpal: Option<Cpal>,
    pub svg: Option<Svg>,
    pub raw_tables: Vec<(Tag, Vec<u8>)>,
}

impl Font {
    pub fn read(buf: &[u8]) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, 0);
        let sfnt_version_val = p.u32()?;
        let sfnt_version = match sfnt_version_val {
            0x00010000 => SfntVersion::TrueType,
            0x4F54544F => SfntVersion::Cff,
            _ => return Err(FontError::invalid_table(Tag::new(b"sfnt"), "invalid sfnt version")),
        };
        let num_tables = p.u16()?;
        let _search_range = p.u16()?;
        let _entry_selector = p.u16()?;
        let _range_shift = p.u16()?;

        let mut tables = Vec::with_capacity(num_tables as usize);
        for _ in 0..num_tables {
            tables.push(TableRecord {
                tag: p.tag()?,
                checksum: p.u32()?,
                offset: p.u32()?,
                length: p.u32()?,
            });
        }

        let find = |tag: Tag| -> Result<&TableRecord, FontError> {
            tables.iter().find(|t| t.tag == tag)
                .ok_or(FontError::MissingTable(tag))
        };

        let head = Head::parse(buf, find(Head::tag())?.offset as usize)?;
        let hhea = Hhea::parse(buf, find(Hhea::tag())?.offset as usize)?;
        let maxp = Maxp::parse(buf, find(Maxp::tag())?.offset as usize)?;
        let post = Post::parse(buf, find(Post::tag())?.offset as usize)?;
        let name = Name::parse(buf, find(Name::tag())?.offset as usize)?;
        let cmap = Cmap::parse(buf, find(Cmap::tag())?.offset as usize)?;
        let os2 = Os2::parse(buf, find(Os2::tag())?.offset as usize)?;

        let hmtx_rec = find(Hmtx::tag())?;
        let hmtx = Hmtx::parse_with_count(buf, hmtx_rec.offset as usize, hhea.number_of_hmetrics, maxp.num_glyphs)?;

        let mut kern: Option<Kern> = None;
        if let Ok(kern_rec) = find(Kern::tag()) {
            kern = Some(Kern::parse(buf, kern_rec.offset as usize)?);
        }

        let cvt = find(Tag::new(b"cvt ")).ok().map(|rec| {
            Parser::new(buf, rec.offset as usize).slice(rec.length as usize).unwrap_or(&[]).to_vec()
        });
        let prep = find(Tag::new(b"prep")).ok().map(|rec| {
            Parser::new(buf, rec.offset as usize).slice(rec.length as usize).unwrap_or(&[]).to_vec()
        });
        let fpgm = find(Tag::new(b"fpgm")).ok().map(|rec| {
            Parser::new(buf, rec.offset as usize).slice(rec.length as usize).unwrap_or(&[]).to_vec()
        });

        let gpos = find(Gpos::tag()).ok().and_then(|rec| {
            let data = Parser::new(buf, rec.offset as usize).slice(rec.length as usize).ok()?;
            Gpos::parse(data).ok()
        });
        let gsub = find(Gsub::tag()).ok().and_then(|rec| {
            let data = Parser::new(buf, rec.offset as usize).slice(rec.length as usize).ok()?;
            Gsub::parse(data).ok()
        });

        let hvar = find(Hvar::tag()).ok().and_then(|rec| {
            let data = Parser::new(buf, rec.offset as usize).slice(rec.length as usize).ok()?;
            Hvar::parse(data).ok()
        });
        let gvar = find(Gvar::tag()).ok().and_then(|rec| {
            let data = Parser::new(buf, rec.offset as usize).slice(rec.length as usize).ok()?;
            Gvar::parse(data).ok()
        });
        let fvar = find(Fvar::tag()).ok().and_then(|rec| {
            let data = Parser::new(buf, rec.offset as usize).slice(rec.length as usize).ok()?;
            Fvar::parse(data).ok()
        });
        let stat = find(Stat::tag()).ok().and_then(|rec| {
            let data = Parser::new(buf, rec.offset as usize).slice(rec.length as usize).ok()?;
            Stat::parse(data).ok()
        });
        let cff = find(Cff::tag()).ok().and_then(|rec| {
            let data = Parser::new(buf, rec.offset as usize).slice(rec.length as usize).ok()?;
            Cff::parse(data).ok()
        });
        let colr = find(Colr::tag()).ok().and_then(|rec| {
            Colr::parse(buf, rec.offset as usize).ok()
        });
        let cpal = find(Cpal::tag()).ok().and_then(|rec| {
            Cpal::parse(buf, rec.offset as usize).ok()
        });
        let svg = find(Svg::tag()).ok().and_then(|rec| {
            Svg::parse(buf, rec.offset as usize).ok()
        });

        let mut glyf: Option<GlyfTable> = None;
        let mut loca: Option<LocaTable> = None;

        if let Ok(glyf_rec) = find(GlyfTable::tag())
            && let Ok(loca_rec) = find(LocaTable::tag()) {
                let num_glyphs = maxp.num_glyphs as usize;
                let long_format = head.index_to_loc_format == 1;
                loca = Some(parse_loca(buf, loca_rec.offset as usize, num_glyphs, long_format)?);
                if let Some(ref loca_table) = loca {
                    glyf = Some(GlyfTable::parse(buf, loca_table, glyf_rec.offset as usize)?);
                }
            }

        let mut raw_tables = Vec::new();
        for rec in &tables {
            let known = [Head::tag(), Hhea::tag(), Maxp::tag(), Post::tag(), Name::tag(), Cmap::tag(), Os2::tag(), GlyfTable::tag(), LocaTable::tag(), Hmtx::tag(), Kern::tag(), Tag::new(b"cvt "), Tag::new(b"prep"), Tag::new(b"fpgm"), Gpos::tag(), Gsub::tag(), Hvar::tag(), Gvar::tag(), Fvar::tag(), Stat::tag(), Cff::tag()];
            if !known.contains(&rec.tag) {
                let data = Parser::new(buf, rec.offset as usize).slice(rec.length as usize)?;
                raw_tables.push((rec.tag, data.to_vec()));
            }
        }

        Ok(Font {
            sfnt_version,
            tables,
            head,
            hhea,
            maxp,
            post,
            name,
            cmap,
            os2,
            glyf,
            loca,
            hmtx,
            kern,
            cvt,
            prep,
            fpgm,
            gpos,
            gsub,
            hvar,
            gvar,
            fvar,
            stat,
            cff,
            colr,
            cpal,
            svg,
            raw_tables,
        })
    }

    pub fn write(&self) -> Result<Vec<u8>, FontError> {
        let mut table_data: Vec<(Tag, Vec<u8>)> = Vec::new();

        let mut head = self.head.clone();
        if self.glyf.is_some() {
            head.index_to_loc_format = 1; // long offsets
        }
        let mut w = Writer::new();
        head.write(&mut w)?;
        table_data.push((Head::tag(), w.into_vec()));

        let mut w = Writer::new();
        self.hhea.write(&mut w)?;
        table_data.push((Hhea::tag(), w.into_vec()));

        let mut w = Writer::new();
        self.maxp.write(&mut w)?;
        table_data.push((Maxp::tag(), w.into_vec()));

        let mut w = Writer::new();
        self.post.write(&mut w)?;
        table_data.push((Post::tag(), w.into_vec()));

        let mut w = Writer::new();
        self.name.write(&mut w)?;
        table_data.push((Name::tag(), w.into_vec()));

        let mut w = Writer::new();
        self.cmap.write(&mut w)?;
        table_data.push((Cmap::tag(), w.into_vec()));

        let mut w = Writer::new();
        self.os2.write(&mut w)?;
        table_data.push((Os2::tag(), w.into_vec()));

        let mut w = Writer::new();
        self.hmtx.write(&mut w)?;
        table_data.push((Hmtx::tag(), w.into_vec()));

        if let Some(kern) = &self.kern {
            let mut w = Writer::new();
            kern.write(&mut w)?;
            table_data.push((Kern::tag(), w.into_vec()));
        }

        if let Some(cvt) = &self.cvt {
            table_data.push((Tag::new(b"cvt "), cvt.clone()));
        }
        if let Some(prep) = &self.prep {
            table_data.push((Tag::new(b"prep"), prep.clone()));
        }
        if let Some(fpgm) = &self.fpgm {
            table_data.push((Tag::new(b"fpgm"), fpgm.clone()));
        }

        if let Some(gpos) = &self.gpos {
            let mut w = Writer::new();
            gpos.write(&mut w)?;
            table_data.push((Gpos::tag(), w.into_vec()));
        }
        if let Some(gsub) = &self.gsub {
            let mut w = Writer::new();
            gsub.write(&mut w)?;
            table_data.push((Gsub::tag(), w.into_vec()));
        }

        if let Some(hvar) = &self.hvar {
            let mut w = Writer::new();
            hvar.write(&mut w)?;
            table_data.push((Hvar::tag(), w.into_vec()));
        }
        if let Some(gvar) = &self.gvar {
            let mut w = Writer::new();
            gvar.write(&mut w)?;
            table_data.push((Gvar::tag(), w.into_vec()));
        }
        if let Some(fvar) = &self.fvar {
            let mut w = Writer::new();
            fvar.write(&mut w)?;
            table_data.push((Fvar::tag(), w.into_vec()));
        }
        if let Some(stat) = &self.stat {
            let mut w = Writer::new();
            stat.write(&mut w)?;
            table_data.push((Stat::tag(), w.into_vec()));
        }
        if let Some(cff) = &self.cff {
            let mut w = Writer::new();
            cff.write(&mut w)?;
            table_data.push((Cff::tag(), w.into_vec()));
        }
        if let Some(colr) = &self.colr {
            let mut w = Writer::new();
            colr.write(&mut w)?;
            table_data.push((Colr::tag(), w.into_vec()));
        }
        if let Some(cpal) = &self.cpal {
            let mut w = Writer::new();
            cpal.write(&mut w)?;
            table_data.push((Cpal::tag(), w.into_vec()));
        }
        if let Some(svg) = &self.svg {
            let mut w = Writer::new();
            svg.write(&mut w)?;
            table_data.push((Svg::tag(), w.into_vec()));
        }

        if let Some(glyf) = &self.glyf {
            let mut glyf_w = Writer::new();
            let mut glyph_sizes = Vec::with_capacity(glyf.glyphs.len());
            for glyph in &glyf.glyphs {
                let start = glyf_w.len();
                glyph.write(&mut glyf_w);
                glyph_sizes.push(glyf_w.len() - start);
            }
            table_data.push((GlyfTable::tag(), glyf_w.into_vec()));

            let new_loca = LocaTable::from_glyph_sizes(&glyph_sizes, true);
            let mut w = Writer::new();
            new_loca.write(&mut w)?;
            table_data.push((LocaTable::tag(), w.into_vec()));
        }

        for (tag, data) in &self.raw_tables {
            table_data.push((*tag, data.clone()));
        }

        // Pad each table to 4-byte boundary
        for (_, data) in &mut table_data {
            while data.len() % 4 != 0 {
                data.push(0);
            }
        }

        // Sort by tag for deterministic output
        table_data.sort_by_key(|a| a.0 .0);

        let num_tables = table_data.len() as u16;
        let search_range = 1u16 << (num_tables as u32).ilog2();
        let entry_selector = (num_tables as u32).ilog2() as u16;
        let range_shift = num_tables * 16 - search_range;

        let header_size = 12 + num_tables as usize * 16;
        let mut offset = header_size as u32;
        let mut records = Vec::with_capacity(table_data.len());
        for (tag, data) in &table_data {
            let checksum = calc_checksum(data);
            records.push(TableRecord {
                tag: *tag,
                checksum,
                offset,
                length: data.len() as u32,
            });
            offset += data.len() as u32;
        }

        let mut w = Writer::new();
        match self.sfnt_version {
            SfntVersion::TrueType => w.write_u32(0x00010000),
            SfntVersion::Cff => w.write_u32(0x4F54544F),
        }
        w.write_u16(num_tables);
        w.write_u16(search_range);
        w.write_u16(entry_selector);
        w.write_u16(range_shift);

        for rec in &records {
            w.write_tag(&rec.tag.0);
            w.write_u32(rec.checksum);
            w.write_u32(rec.offset);
            w.write_u32(rec.length);
        }

        for (_, data) in &table_data {
            w.write_bytes(data);
        }

        let mut buf = w.into_vec();
        // Update head.checkSumAdjustment
        let whole_checksum = calc_checksum(&buf);
        let checksum_adjustment = 0xB1B0AFBAu32.wrapping_sub(whole_checksum);
        // head table is first in the data after header (since we sorted)
        // Actually we need to find where head is in the records
        if let Some(head_rec) = records.iter().find(|r| r.tag == Head::tag()) {
            let head_offset = head_rec.offset as usize;
            // checkSumAdjustment is at offset 8 within head table
            buf[head_offset + 8..head_offset + 12].copy_from_slice(&checksum_adjustment.to_be_bytes());
        }

        Ok(buf)
    }
}

impl Font {
    pub fn create_minimal() -> Self {
        use crate::tables::name::NameRecord;
        use crate::tables::cmap::{Cmap, CmapSubtable, SequentialMapGroup};
        use crate::tables::glyf::{GlyfTable, Glyph};

        let glyf = GlyfTable { glyphs: vec![Glyph::Empty] };
        let loca = LocaTable::from_glyph_sizes(&[0], false);
        let hmtx = Hmtx {
            h_metrics: vec![crate::tables::hmtx::LongHorMetricRecord { advance_width: 500, lsb: 0 }],
            left_side_bearings: vec![],
        };

        Font {
            sfnt_version: SfntVersion::TrueType,
            tables: vec![],
            head: Head {
                major_version: 1,
                minor_version: 0,
                font_revision: 0x00010000,
                check_sum_adjustment: 0,
                magic_number: 0x5F0F3CF5,
                flags: 0,
                units_per_em: 1000,
                created: 0,
                modified: 0,
                x_min: 0,
                y_min: 0,
                x_max: 500,
                y_max: 700,
                mac_style: 0,
                lowest_rec_ppem: 3,
                font_direction_hint: 2,
                index_to_loc_format: 1,
                glyph_data_format: 0,
            },
            hhea: Hhea {
                major_version: 1,
                minor_version: 0,
                ascender: 800,
                descender: -200,
                line_gap: 0,
                advance_width_max: 500,
                min_left_side_bearing: 0,
                min_right_side_bearing: 0,
                x_max_extent: 500,
                caret_slope_rise: 1,
                caret_slope_run: 0,
                caret_offset: 0,
                reserved: [0; 4],
                metric_data_format: 0,
                number_of_hmetrics: 1,
            },
            maxp: Maxp {
                version: 0x00005000,
                num_glyphs: 1,
                max_points: None,
                max_contours: None,
                max_composite_points: None,
                max_composite_contours: None,
                max_zones: None,
                max_twilight_points: None,
                max_storage: None,
                max_function_defs: None,
                max_instruction_defs: None,
                max_stack_elements: None,
                max_size_of_instructions: None,
                max_component_elements: None,
                max_component_depth: None,
            },
            post: Post {
                version: 0x00030000,
                italic_angle: 0,
                underline_position: 0,
                underline_thickness: 0,
                is_fixed_pitch: 0,
                min_mem_type42: 0,
                max_mem_type42: 0,
                min_mem_type1: 0,
                max_mem_type1: 0,
                names: None,
            },
            name: Name {
                format: 0,
                count: 2,
                string_offset: 0,
                records: vec![
                    NameRecord {
                        platform_id: 1,
                        encoding_id: 0,
                        language_id: 0,
                        name_id: 1,
                        string: "TestFont".to_string(),
                    },
                    NameRecord {
                        platform_id: 1,
                        encoding_id: 0,
                        language_id: 0,
                        name_id: 2,
                        string: "Regular".to_string(),
                    },
                ],
            },
            cmap: Cmap {
                version: 0,
                num_tables: 1,
                records: vec![crate::tables::cmap::EncodingRecord {
                    platform_id: 3,
                    encoding_id: 1,
                    subtable_offset: 0,
                }],
                subtables: vec![CmapSubtable::Format12 {
                    language: 0,
                    groups: vec![SequentialMapGroup {
                        start_char_code: 0x20,
                        end_char_code: 0x7E,
                        start_glyph_id: 1,
                    }],
                }],
            },
            os2: Os2 {
                version: 4,
                x_avg_char_width: 500,
                us_weight_class: 400,
                us_width_class: 5,
                fs_type: 0,
                y_subscript_x_size: 0,
                y_subscript_y_size: 0,
                y_subscript_x_offset: 0,
                y_subscript_y_offset: 0,
                y_superscript_x_size: 0,
                y_superscript_y_size: 0,
                y_superscript_x_offset: 0,
                y_superscript_y_offset: 0,
                y_strikeout_size: 0,
                y_strikeout_position: 0,
                s_family_class: 0,
                panose: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                ul_unicode_range1: 0,
                ul_unicode_range2: 0,
                ul_unicode_range3: 0,
                ul_unicode_range4: 0,
                ach_vend_id: [0, 0, 0, 0],
                fs_selection: 0x0040,
                us_first_char_index: 0x0020,
                us_last_char_index: 0x007E,
                s_typo_ascender: 800,
                s_typo_descender: -200,
                s_typo_line_gap: 0,
                us_win_ascent: 1000,
                us_win_descent: 200,
                ul_code_page_range1: Some(1),
                ul_code_page_range2: Some(0),
                sx_height: Some(500),
                s_cap_height: Some(700),
                us_default_char: Some(0),
                us_break_char: Some(0x20),
                us_max_context: Some(0),
                us_lower_optical_point_size: None,
                us_upper_optical_point_size: None,
            },
            glyf: Some(glyf),
            loca: Some(loca),
            hmtx,
            kern: None,
            cvt: None,
            prep: None,
            fpgm: None,
            gpos: None,
            gsub: None,
            hvar: None,
            gvar: None,
            fvar: None,
            stat: None,
            cff: None,
            colr: None,
            cpal: None,
            svg: None,
            raw_tables: vec![],
        }
    }
}

fn parse_loca(buf: &[u8], offset: usize, num_glyphs: usize, long_format: bool) -> Result<LocaTable, FontError> {
    let mut p = Parser::new(buf, offset);
    let mut offsets = Vec::with_capacity(num_glyphs + 1);
    if long_format {
        for _ in 0..=num_glyphs {
            offsets.push(p.u32()?);
        }
    } else {
        for _ in 0..=num_glyphs {
            offsets.push(p.u16()? as u32 * 2);
        }
    }
    Ok(LocaTable { offsets, format: if long_format { 1 } else { 0 } })
}

fn calc_checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 4 <= data.len() {
        sum = sum.wrapping_add(u32::from_be_bytes([
            data[i], data[i + 1], data[i + 2], data[i + 3],
        ]));
        i += 4;
    }
    sum
}

impl Font {
    /// Return a new font containing only the glyphs with the given IDs.
    pub fn subset(&self, keep_glyphs: &[u16]) -> Self {
        let mut new_font = self.clone();

        // Rebuild glyf
        if let (Some(glyf), Some(_loca)) = (&self.glyf, &self.loca) {
            let mut new_glyphs = Vec::with_capacity(keep_glyphs.len());
            let mut new_hmetrics = Vec::with_capacity(keep_glyphs.len());
            let mut new_lsbs = Vec::new();

            for &gid in keep_glyphs {
                let idx = gid as usize;
                if idx < glyf.glyphs.len() {
                    new_glyphs.push(glyf.glyphs[idx].clone());
                }
                let (aw, lsb) = self.hmtx.metric_for_glyph(gid);
                if new_hmetrics.len() < keep_glyphs.len() {
                    new_hmetrics.push(crate::tables::hmtx::LongHorMetricRecord { advance_width: aw, lsb });
                } else {
                    new_lsbs.push(lsb);
                }
            }

            let num_hmetrics = new_hmetrics.len().min(new_glyphs.len()) as u16;
            new_font.hhea.number_of_hmetrics = num_hmetrics;
            new_font.maxp.num_glyphs = new_glyphs.len() as u16;
            new_font.hmtx = crate::tables::hmtx::Hmtx {
                h_metrics: new_hmetrics,
                left_side_bearings: new_lsbs,
            };

            let sizes: Vec<usize> = new_glyphs.iter().map(|g| {
                let mut w = Writer::new();
                g.write(&mut w);
                w.len()
            }).collect();
            new_font.glyf = Some(crate::tables::glyf::GlyfTable { glyphs: new_glyphs });
            new_font.loca = Some(crate::tables::loca::LocaTable::from_glyph_sizes(&sizes, true));
        }

        // Rebuild kern: keep only pairs where both glyphs are in keep_glyphs
        if let Some(ref kern) = self.kern {
            let keep_set: std::collections::HashSet<u16> = keep_glyphs.iter().copied().collect();
            let mut new_subtables = Vec::new();
            for sub in &kern.subtables {
                let new_pairs: Vec<crate::tables::kern::KernPair> = sub.pairs.iter()
                    .filter(|p| keep_set.contains(&p.left) && keep_set.contains(&p.right))
                    .cloned()
                    .collect();
                if !new_pairs.is_empty() {
                    new_subtables.push(crate::tables::kern::KernSubtable {
                        version: sub.version,
                        length: 0, // recalculated on write
                        coverage: sub.coverage,
                        pairs: new_pairs,
                    });
                }
            }
            if !new_subtables.is_empty() {
                new_font.kern = Some(crate::tables::kern::Kern {
                    version: kern.version,
                    n_tables: new_subtables.len() as u16,
                    subtables: new_subtables,
                });
            } else {
                new_font.kern = None;
            }
        }

        // GPOS/GSUB/fvar/stat/cff/colr/cpal/svg glyph IDs are invalidated by subsetting
        new_font.gpos = None;
        new_font.gsub = None;
        new_font.fvar = None;
        new_font.stat = None;
        new_font.cff = None;
        new_font.colr = None;
        new_font.cpal = None;
        new_font.svg = None;

        new_font
    }

    /// Verify table checksums and basic structural consistency, returning the
    /// findings as human-readable strings. For programmatic inspection, use
    /// [`Font::validate_report`](crate::Font::validate_report).
    pub fn validate(&self, buf: &[u8]) -> Vec<String> {
        self.validate_report(buf)
            .issues
            .into_iter()
            .map(|i| i.message)
            .collect()
    }
}
