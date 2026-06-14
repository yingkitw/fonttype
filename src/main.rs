use std::path::PathBuf;
use clap::{Parser, Subcommand};
use fonttype::Table;

#[derive(Parser)]
#[command(name = "fonttool")]
#[command(about = "Read and write TrueType / OpenType font files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print font metadata summary
    Info { file: PathBuf },
    /// Dump parsed tables
    Dump { file: PathBuf },
    /// Create a minimal test font
    Create { out: PathBuf },
    /// Create a font from a bitmap image (traces dark regions as glyph outlines)
    CreateFromImage {
        image: PathBuf,
        codepoint: u32,
        out: PathBuf,
    },
    /// Export a glyph to a PNG image
    ExportToImage {
        font: PathBuf,
        glyph_id: u16,
        out: PathBuf,
        #[arg(short, long, default_value = "256")]
        size: u32,
    },
    /// Subset a font to keep only specified glyph IDs
    Subset {
        font: PathBuf,
        glyph_ids: Vec<u16>,
        out: PathBuf,
    },
    /// Validate a font file
    Validate {
        font: PathBuf,
    },
    /// Convert TTF/OTF to WOFF
    ToWoff {
        font: PathBuf,
        out: PathBuf,
    },
    /// Convert WOFF to TTF/OTF
    FromWoff {
        font: PathBuf,
        out: PathBuf,
    },
    /// Merge two fonts (append glyphs from second to first)
    Merge {
        base: PathBuf,
        append: PathBuf,
        out: PathBuf,
    },
    /// List all tables in the font
    Tables {
        file: PathBuf,
    },
    /// Query cmap mapping: codepoint → glyph ID or reverse
    Map {
        file: PathBuf,
        /// Codepoint in hex (e.g. 0x41) or glyph ID (e.g. gid:1)
        query: String,
    },
    /// Show font statistics
    Stats {
        file: PathBuf,
    },
    /// Rewrite font with correct checksums and alignment
    Fix {
        font: PathBuf,
        out: PathBuf,
    },
    /// Extract a single table to a binary file
    Extract {
        font: PathBuf,
        table: String,
        out: PathBuf,
    },
    /// Inject or replace a table from a binary file
    Inject {
        font: PathBuf,
        table: String,
        data: PathBuf,
        out: PathBuf,
    },
    /// Rename family and/or subfamily in the name table
    Rename {
        font: PathBuf,
        out: PathBuf,
        #[arg(short, long)]
        family: Option<String>,
        #[arg(short, long)]
        subfamily: Option<String>,
    },
    /// Remove hinting tables (cvt, prep, fpgm) to reduce file size
    Strip {
        font: PathBuf,
        out: PathBuf,
    },
    /// Report Unicode block coverage
    Coverage {
        file: PathBuf,
    },
    /// Structural diff between two fonts
    Compare {
        font_a: PathBuf,
        font_b: PathBuf,
    },
    /// Convert sfnt font to WOFF2
    ToWoff2 {
        font: PathBuf,
        out: PathBuf,
    },
    /// Convert WOFF2 font to sfnt
    FromWoff2 {
        font: PathBuf,
        out: PathBuf,
    },
    /// List fonts in a TrueType Collection
    TtcInfo {
        file: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Info { file } => {
            let bytes = std::fs::read(&file)?;
            let font = fonttype::Font::read(&bytes)?;
            println!("File: {}", file.display());
            println!("  Tables: {}", font.tables.len());
            println!("  Family: {}", font.name.family_name().unwrap_or_else(|| "?".into()));
            println!("  Subfamily: {}", font.name.subfamily_name().unwrap_or_else(|| "?".into()));
            println!("  Full name: {}", font.name.full_name().unwrap_or_else(|| "?".into()));
            println!("  Version: {}", font.name.version().unwrap_or_else(|| "?".into()));
            println!("  Units per em: {}", font.head.units_per_em);
            println!("  Num glyphs: {}", font.maxp.num_glyphs);
            println!("  Ascender: {}", font.hhea.ascender);
            println!("  Descender: {}", font.hhea.descender);
            println!("  Advance width max: {}", font.hhea.advance_width_max);
            if let Some(ref gpos) = font.gpos {
                println!("  GPOS kerning pairs: {}", gpos.kerning.len());
            }
            if let Some(ref gsub) = font.gsub {
                println!("  GSUB features: {}", gsub.features.join(", "));
                println!("  Has ligatures: {}", gsub.has_ligatures());
            }
            if let Some(ref fvar) = font.fvar {
                let axes: Vec<String> = fvar.axes.iter().map(|a| format!("{} ({:.0}-{:.0})", a.axis_tag, a.min_value, a.max_value)).collect();
                println!("  Variable axes: {}", axes.join(", "));
            }
            if let Some(ref stat) = font.stat {
                let axes: Vec<String> = stat.design_axes.iter().map(|a| a.axis_tag.to_string()).collect();
                println!("  STAT axes: {}", axes.join(", "));
                println!("  STAT axis values: {}", stat.axis_values.len());
            }
        }
        Commands::Dump { file } => {
            let bytes = std::fs::read(&file)?;
            let font = fonttype::Font::read(&bytes)?;
            println!("{:#?}", font);
        }
        Commands::Create { out } => {
            let font = fonttype::Font::create_minimal();
            std::fs::write(&out, font.write()?)?;
            println!("Created {}", out.display());
        }
        Commands::CreateFromImage { image, codepoint, out } => {
            let img = image::open(&image)?.to_luma8();
            let contours = fonttype::image::tracer::trace_image(&img, 128);
            let glyph = fonttype::tables::glyf::Glyph::from_points(contours);
            let mut font = fonttype::Font::create_minimal();
            // Update font with new glyph
            if let Some(ref mut glyf) = font.glyf {
                glyf.glyphs.push(glyph);
            }
            if let Some(ref mut loca) = font.loca {
                let mut sizes = Vec::new();
                if let Some(ref glyf) = font.glyf {
                    for g in &glyf.glyphs {
                        let mut w = fonttype::write::Writer::new();
                        g.write(&mut w);
                        sizes.push(w.len());
                    }
                }
                *loca = fonttype::tables::loca::LocaTable::from_glyph_sizes(&sizes, true);
            }
            // Update hmtx for 2 glyphs
            font.hmtx = fonttype::tables::hmtx::Hmtx {
                h_metrics: vec![
                    fonttype::tables::hmtx::LongHorMetricRecord { advance_width: 500, lsb: 0 },
                    fonttype::tables::hmtx::LongHorMetricRecord { advance_width: 500, lsb: 0 },
                ],
                left_side_bearings: vec![],
            };
            font.maxp.num_glyphs = 2;
            font.hhea.number_of_hmetrics = 2;
            // Update cmap to map codepoint to glyph 1
            if let Some(ref mut cmap) = font.cmap.subtables.first_mut() {
                match cmap {
                    fonttype::tables::cmap::CmapSubtable::Format12 { groups, .. } => {
                        groups.push(fonttype::tables::cmap::SequentialMapGroup {
                            start_char_code: codepoint,
                            end_char_code: codepoint,
                            start_glyph_id: 1,
                        });
                    }
                    _ => {}
                }
            }
            std::fs::write(&out, font.write()?)?;
            println!("Created {} from {} for U+{:04X}", out.display(), image.display(), codepoint);
        }
        Commands::ExportToImage { font, glyph_id, out, size } => {
            let bytes = std::fs::read(&font)?;
            let f = fonttype::Font::read(&bytes)?;
            if let Some(ref glyf) = f.glyf {
                if let Some(g) = glyf.glyphs.get(glyph_id as usize) {
                    if let fonttype::tables::glyf::Glyph::Simple(sg) = g {
                        fonttype::image::rasterizer::export_glyph_to_image(sg, &out, size)?;
                        println!("Exported glyph {} to {} ({}x{})", glyph_id, out.display(), size, size);
                    } else {
                        println!("Glyph {} is not a simple glyph", glyph_id);
                    }
                } else {
                    println!("Glyph {} not found", glyph_id);
                }
            } else {
                println!("No glyf table in font");
            }
        }
        Commands::Subset { font, glyph_ids, out } => {
            let bytes = std::fs::read(&font)?;
            let f = fonttype::Font::read(&bytes)?;
            let subset = f.subset(&glyph_ids);
            std::fs::write(&out, subset.write()?)?;
            println!("Subset {} -> {} (kept {} glyphs)", font.display(), out.display(), glyph_ids.len());
        }
        Commands::Validate { font } => {
            let bytes = std::fs::read(&font)?;
            let f = fonttype::Font::read(&bytes)?;
            let issues = f.validate(&bytes);
            if issues.is_empty() {
                println!("{}: valid", font.display());
            } else {
                println!("{}: {} issue(s)", font.display(), issues.len());
                for issue in &issues {
                    println!("  - {}", issue);
                }
            }
        }
        Commands::ToWoff { font, out } => {
            let bytes = std::fs::read(&font)?;
            let f = fonttype::Font::read(&bytes)?;
            let sfnt = f.write()?;
            // Parse sfnt to extract table data for WOFF
            let mut p = fonttype::parse::Parser::new(&sfnt, 0);
            let _sfnt_version = p.u32()?;
            let num_tables = p.u16()?;
            let _search_range = p.u16()?;
            let _entry_selector = p.u16()?;
            let _range_shift = p.u16()?;
            let mut table_data: Vec<(fonttype::Tag, Vec<u8>)> = Vec::with_capacity(num_tables as usize);
            for _ in 0..num_tables {
                let tag = p.tag()?;
                let _checksum = p.u32()?;
                let offset = p.u32()? as usize;
                let length = p.u32()? as usize;
                let data = p.buf()[offset..offset + length].to_vec();
                table_data.push((tag, data));
            }
            let woff = fonttype::write_woff(&table_data)?;
            std::fs::write(&out, woff)?;
            println!("Converted {} -> {} (WOFF)", font.display(), out.display());
        }
        Commands::FromWoff { font, out } => {
            let bytes = std::fs::read(&font)?;
            let sfnt = fonttype::read_woff(&bytes)?;
            std::fs::write(&out, sfnt)?;
            println!("Converted {} -> {} (sfnt)", font.display(), out.display());
        }
        Commands::Merge { base, append, out } => {
            let base_bytes = std::fs::read(&base)?;
            let append_bytes = std::fs::read(&append)?;
            let mut base_font = fonttype::Font::read(&base_bytes)?;
            let append_font = fonttype::Font::read(&append_bytes)?;

            let base_glyph_count = base_font.maxp.num_glyphs as u16;
            if let (Some(base_glyf), Some(append_glyf)) = (&mut base_font.glyf, &append_font.glyf) {
                for g in &append_glyf.glyphs {
                    base_glyf.glyphs.push(g.clone());
                }
                let sizes: Vec<usize> = base_glyf.glyphs.iter().map(|g| {
                    let mut w = fonttype::write::Writer::new();
                    g.write(&mut w);
                    w.len()
                }).collect();
                base_font.loca = Some(fonttype::tables::loca::LocaTable::from_glyph_sizes(&sizes, true));
                base_font.maxp.num_glyphs = base_glyf.glyphs.len() as u16;
            }
            for rec in &append_font.hmtx.h_metrics {
                base_font.hmtx.h_metrics.push(rec.clone());
            }
            for lsb in &append_font.hmtx.left_side_bearings {
                base_font.hmtx.left_side_bearings.push(*lsb);
            }
            base_font.hhea.number_of_hmetrics = base_font.hmtx.h_metrics.len() as u16;
            let append_cmap = &append_font.cmap;
            if let (Some(base_sub), Some(append_sub)) = (base_font.cmap.subtables.first_mut(), append_cmap.subtables.first()) {
                if let (fonttype::tables::cmap::CmapSubtable::Format12 { groups: base_groups, .. },
                        fonttype::tables::cmap::CmapSubtable::Format12 { groups: append_groups, .. }) = (base_sub, append_sub) {
                    for group in append_groups {
                        base_groups.push(fonttype::tables::cmap::SequentialMapGroup {
                            start_char_code: group.start_char_code,
                            end_char_code: group.end_char_code,
                            start_glyph_id: group.start_glyph_id + base_glyph_count as u32,
                        });
                    }
                }
            }
            // GPOS/GSUB/kern/hvar/gvar invalidated by merge
            base_font.gpos = None;
            base_font.gsub = None;
            base_font.kern = None;
            base_font.hvar = None;
            base_font.gvar = None;

            std::fs::write(&out, base_font.write()?)?;
            println!("Merged {} + {} -> {}", base.display(), append.display(), out.display());
        }
        Commands::Tables { file } => {
            let bytes = std::fs::read(&file)?;
            let font = fonttype::Font::read(&bytes)?;
            println!("{:<8} {:>10} {:>10} {:>10}", "Tag", "Offset", "Length", "Checksum");
            for rec in &font.tables {
                println!("{:<8} {:>10} {:>10} {:>10}", rec.tag, rec.offset, rec.length, rec.checksum);
            }
        }
        Commands::Map { file, query } => {
            let bytes = std::fs::read(&file)?;
            let font = fonttype::Font::read(&bytes)?;
            if let Some(gid_str) = query.strip_prefix("gid:") {
                let gid: u16 = gid_str.parse()?;
                let cps = font.cmap.glyph_codepoints(gid);
                if cps.is_empty() {
                    println!("Glyph {} has no mapped codepoints", gid);
                } else {
                    println!("Glyph {} maps to:", gid);
                    for cp in cps {
                        let ch = std::char::from_u32(cp);
                        if let Some(ch) = ch {
                            println!("  U+{:04X} {}", cp, ch);
                        } else {
                            println!("  U+{:04X}", cp);
                        }
                    }
                }
            } else {
                let cp = if query.starts_with("0x") || query.starts_with("0X") {
                    u32::from_str_radix(&query[2..], 16)?
                } else {
                    let chars: Vec<char> = query.chars().collect();
                    if chars.len() == 1 {
                        chars[0] as u32
                    } else {
                        query.parse::<u32>()?
                    }
                };
                if let Some(gid) = font.cmap.map_codepoint(cp) {
                    let ch = std::char::from_u32(cp);
                    if let Some(ch) = ch {
                        println!("U+{:04X} ({}) → glyph {}", cp, ch, gid);
                    } else {
                        println!("U+{:04X} → glyph {}", cp, gid);
                    }
                } else {
                    println!("U+{:04X} not mapped", cp);
                }
            }
        }
        Commands::Stats { file } => {
            let bytes = std::fs::read(&file)?;
            let font = fonttype::Font::read(&bytes)?;
            println!("File: {}", file.display());
            println!("  File size: {} bytes", bytes.len());
            println!("  Tables: {}", font.tables.len());
            println!("  Glyphs: {}", font.maxp.num_glyphs);
            println!("  Units per em: {}", font.head.units_per_em);
            println!("  Ascender: {}", font.hhea.ascender);
            println!("  Descender: {}", font.hhea.descender);
            println!("  Advance width max: {}", font.hhea.advance_width_max);
            if let Some(ref glyf) = font.glyf {
                let simple = glyf.glyphs.iter().filter(|g| matches!(g, fonttype::tables::glyf::Glyph::Simple(_))).count();
                let composite = glyf.glyphs.iter().filter(|g| matches!(g, fonttype::tables::glyf::Glyph::Composite(_))).count();
                let empty = glyf.glyphs.iter().filter(|g| matches!(g, fonttype::tables::glyf::Glyph::Empty)).count();
                println!("  Simple glyphs: {}", simple);
                println!("  Composite glyphs: {}", composite);
                println!("  Empty glyphs: {}", empty);
            }
            let mut table_total = 0usize;
            for rec in &font.tables {
                table_total += rec.length as usize;
            }
            println!("  Table data total: {} bytes", table_total);
            println!("  Overhead: {} bytes", bytes.len() - table_total);
        }
        Commands::Fix { font, out } => {
            let bytes = std::fs::read(&font)?;
            let f = fonttype::Font::read(&bytes)?;
            std::fs::write(&out, f.write()?)?;
            println!("Fixed {} -> {}", font.display(), out.display());
        }
        Commands::Extract { font, table, out } => {
            let bytes = std::fs::read(&font)?;
            let font_obj = fonttype::Font::read(&bytes)?;
            let tag_bytes: [u8; 4] = table.as_bytes().try_into().unwrap_or_else(|_| {
                eprintln!("Table tag must be exactly 4 ASCII characters, got '{}'", table);
                std::process::exit(1);
            });
            let tag = fonttype::Tag::new(&tag_bytes);
            if let Some(rec) = font_obj.tables.iter().find(|t| t.tag == tag) {
                let start = rec.offset as usize;
                let end = start + rec.length as usize;
                std::fs::write(&out, &bytes[start..end])?;
                println!("Extracted {} ({} bytes) from {} -> {}", table, rec.length, font.display(), out.display());
            } else {
                eprintln!("Table {} not found in {}", table, font.display());
                std::process::exit(1);
            }
        }
        Commands::Inject { font, table, data, out } => {
            let mut font_obj = fonttype::Font::read(&std::fs::read(&font)?)?;
            let tag_bytes: [u8; 4] = table.as_bytes().try_into().unwrap_or_else(|_| {
                eprintln!("Table tag must be exactly 4 ASCII characters, got '{}'", table);
                std::process::exit(1);
            });
            let tag = fonttype::Tag::new(&tag_bytes);
            let known = [
                fonttype::tables::head::Head::tag(),
                fonttype::tables::hhea::Hhea::tag(),
                fonttype::tables::maxp::Maxp::tag(),
                fonttype::tables::post::Post::tag(),
                fonttype::tables::name::Name::tag(),
                fonttype::tables::cmap::Cmap::tag(),
                fonttype::tables::os2::Os2::tag(),
                fonttype::tables::glyf::GlyfTable::tag(),
                fonttype::tables::loca::LocaTable::tag(),
                fonttype::tables::hmtx::Hmtx::tag(),
                fonttype::tables::kern::Kern::tag(),
                fonttype::Tag::new(b"cvt "),
                fonttype::Tag::new(b"prep"),
                fonttype::Tag::new(b"fpgm"),
                fonttype::tables::gpos::Gpos::tag(),
                fonttype::tables::gsub::Gsub::tag(),
                fonttype::tables::var::Hvar::tag(),
                fonttype::tables::var::Gvar::tag(),
                fonttype::tables::fvar::Fvar::tag(),
            ];
            if known.contains(&tag) {
                eprintln!("Injecting known table '{}' is not yet supported. Use a specific command or modify the source.", table);
                std::process::exit(1);
            }
            let table_data = std::fs::read(&data)?;
            // Remove existing if present
            font_obj.raw_tables.retain(|(t, _)| *t != tag);
            font_obj.raw_tables.push((tag, table_data));
            std::fs::write(&out, font_obj.write()?)?;
            println!("Injected {} into {} -> {}", table, font.display(), out.display());
        }
        Commands::Rename { font, out, family, subfamily } => {
            let mut font_obj = fonttype::Font::read(&std::fs::read(&font)?)?;
            if let Some(ref name) = family {
                font_obj.name.set_family(name);
            }
            if let Some(ref name) = subfamily {
                font_obj.name.set_subfamily(name);
            }
            std::fs::write(&out, font_obj.write()?)?;
            println!("Renamed {} -> {}", font.display(), out.display());
        }
        Commands::Strip { font, out } => {
            let mut font_obj = fonttype::Font::read(&std::fs::read(&font)?)?;
            let before = font_obj.tables.iter().map(|r| r.length as usize).sum::<usize>();
            font_obj.cvt = None;
            font_obj.prep = None;
            font_obj.fpgm = None;
            // Also strip instructions from simple glyphs
            if let Some(ref mut glyf) = font_obj.glyf {
                for glyph in &mut glyf.glyphs {
                    if let fonttype::tables::glyf::Glyph::Simple(sg) = glyph {
                        sg.instructions.clear();
                    }
                }
            }
            std::fs::write(&out, font_obj.write()?)?;
            let after = std::fs::metadata(&out)?.len() as usize;
            println!("Stripped {} -> {} ({} bytes removed)", font.display(), out.display(), before.saturating_sub(after));
        }
        Commands::Coverage { file } => {
            let bytes = std::fs::read(&file)?;
            let font = fonttype::Font::read(&bytes)?;
            let mut cps = Vec::new();
            for subtable in &font.cmap.subtables {
                match subtable {
                    fonttype::tables::cmap::CmapSubtable::Format0 { glyph_id_array, .. } => {
                        for (cp, &gid) in glyph_id_array.iter().enumerate() {
                            if gid != 0 {
                                cps.push(cp as u32);
                            }
                        }
                    }
                    fonttype::tables::cmap::CmapSubtable::Format12 { groups, .. } => {
                        for g in groups {
                            for cp in g.start_char_code..=g.end_char_code {
                                cps.push(cp);
                            }
                        }
                    }
                    fonttype::tables::cmap::CmapSubtable::Format4 { segments, .. } => {
                        for seg in segments {
                            for cp in seg.start_code..=seg.end_code {
                                cps.push(cp as u32);
                            }
                        }
                    }
                }
            }
            cps.sort_unstable();
            cps.dedup();

            // Unicode block definitions: (start, end, name)
            let blocks: Vec<(u32, u32, &str)> = vec![
                (0x0000, 0x007F, "Basic Latin"),
                (0x0080, 0x00FF, "Latin-1 Supplement"),
                (0x0100, 0x017F, "Latin Extended-A"),
                (0x0180, 0x024F, "Latin Extended-B"),
                (0x0250, 0x02AF, "IPA Extensions"),
                (0x02B0, 0x02FF, "Spacing Modifier Letters"),
                (0x0300, 0x036F, "Combining Diacritical Marks"),
                (0x0370, 0x03FF, "Greek and Coptic"),
                (0x0400, 0x04FF, "Cyrillic"),
                (0x0500, 0x052F, "Cyrillic Supplement"),
                (0x0530, 0x058F, "Armenian"),
                (0x0590, 0x05FF, "Hebrew"),
                (0x0600, 0x06FF, "Arabic"),
                (0x0900, 0x097F, "Devanagari"),
                (0x10A0, 0x10FF, "Georgian"),
                (0x1100, 0x11FF, "Hangul Jamo"),
                (0x13A0, 0x13FF, "Cherokee"),
                (0x1400, 0x167F, "Unified Canadian Aboriginal Syllabics"),
                (0x1680, 0x169F, "Ogham"),
                (0x16A0, 0x16FF, "Runic"),
                (0x1780, 0x17FF, "Khmer"),
                (0x1800, 0x18AF, "Mongolian"),
                (0x1E00, 0x1EFF, "Latin Extended Additional"),
                (0x1F00, 0x1FFF, "Greek Extended"),
                (0x2000, 0x206F, "General Punctuation"),
                (0x2070, 0x209F, "Superscripts and Subscripts"),
                (0x20A0, 0x20CF, "Currency Symbols"),
                (0x20D0, 0x20FF, "Combining Diacritical Marks for Symbols"),
                (0x2100, 0x214F, "Letterlike Symbols"),
                (0x2150, 0x218F, "Number Forms"),
                (0x2190, 0x21FF, "Arrows"),
                (0x2200, 0x22FF, "Mathematical Operators"),
                (0x2300, 0x23FF, "Miscellaneous Technical"),
                (0x2400, 0x243F, "Control Pictures"),
                (0x2440, 0x245F, "Optical Character Recognition"),
                (0x2460, 0x24FF, "Enclosed Alphanumerics"),
                (0x2500, 0x257F, "Box Drawing"),
                (0x2580, 0x259F, "Block Elements"),
                (0x25A0, 0x25FF, "Geometric Shapes"),
                (0x2600, 0x26FF, "Miscellaneous Symbols"),
                (0x2700, 0x27BF, "Dingbats"),
                (0x2800, 0x28FF, "Braille Patterns"),
                (0x2E80, 0x2EFF, "CJK Radicals Supplement"),
                (0x2F00, 0x2FDF, "Kangxi Radicals"),
                (0x3000, 0x303F, "CJK Symbols and Punctuation"),
                (0x3040, 0x309F, "Hiragana"),
                (0x30A0, 0x30FF, "Katakana"),
                (0x3100, 0x312F, "Bopomofo"),
                (0x3130, 0x318F, "Hangul Compatibility Jamo"),
                (0x3200, 0x32FF, "Enclosed CJK Letters and Months"),
                (0x3300, 0x33FF, "CJK Compatibility"),
                (0x3400, 0x4DBF, "CJK Unified Ideographs Extension A"),
                (0x4E00, 0x9FFF, "CJK Unified Ideographs"),
                (0xA000, 0xA48F, "Yi Syllables"),
                (0xA490, 0xA4CF, "Yi Radicals"),
                (0xAC00, 0xD7AF, "Hangul Syllables"),
                (0xE000, 0xF8FF, "Private Use Area"),
                (0xF900, 0xFAFF, "CJK Compatibility Ideographs"),
                (0xFB00, 0xFB4F, "Alphabetic Presentation Forms"),
                (0xFB50, 0xFDFF, "Arabic Presentation Forms-A"),
                (0xFE20, 0xFE2F, "Combining Half Marks"),
                (0xFE30, 0xFE4F, "CJK Compatibility Forms"),
                (0xFE50, 0xFE6F, "Small Form Variants"),
                (0xFE70, 0xFEFF, "Arabic Presentation Forms-B"),
                (0xFF00, 0xFFEF, "Halfwidth and Fullwidth Forms"),
                (0xFFFD, 0xFFFD, "Specials"),
                (0x1F300, 0x1F5FF, "Miscellaneous Symbols and Pictographs"),
                (0x1F600, 0x1F64F, "Emoticons"),
                (0x1F680, 0x1F6FF, "Transport and Map Symbols"),
                (0x1F700, 0x1F77F, "Alchemical Symbols"),
                (0x1F900, 0x1F9FF, "Supplemental Symbols and Pictographs"),
            ];
            println!("Coverage for {}:", file.display());
            let mut total = 0usize;
            for (start, end, name) in blocks {
                let count = cps.iter().filter(|&&cp| cp >= start && cp <= end).count();
                if count > 0 {
                    let range_size = (end - start + 1) as usize;
                    let pct = count as f32 / range_size as f32 * 100.0;
                    println!("  {:<48} {:>4}/{} ({:.0}%)", name, count, range_size, pct);
                    total += count;
                }
            }
            println!("  Total mapped codepoints: {}", total);
        }
        Commands::Compare { font_a, font_b } => {
            let a_bytes = std::fs::read(&font_a)?;
            let b_bytes = std::fs::read(&font_b)?;
            let a = fonttype::Font::read(&a_bytes)?;
            let b = fonttype::Font::read(&b_bytes)?;

            let mut diffs = Vec::new();

            // Table presence
            let a_tags: std::collections::HashSet<_> = a.tables.iter().map(|t| t.tag.clone()).collect();
            let b_tags: std::collections::HashSet<_> = b.tables.iter().map(|t| t.tag.clone()).collect();
            for tag in a_tags.difference(&b_tags) {
                diffs.push(format!("Table {} only in {}", tag, font_a.display()));
            }
            for tag in b_tags.difference(&a_tags) {
                diffs.push(format!("Table {} only in {}", tag, font_b.display()));
            }

            // Table sizes
            for tag in a_tags.intersection(&b_tags) {
                let a_rec = a.tables.iter().find(|t| &t.tag == tag).unwrap();
                let b_rec = b.tables.iter().find(|t| &t.tag == tag).unwrap();
                if a_rec.length != b_rec.length {
                    diffs.push(format!("Table {} size: {} vs {}", tag, a_rec.length, b_rec.length));
                }
            }

            // Metadata
            if a.name.family_name() != b.name.family_name() {
                diffs.push(format!("Family: '{}' vs '{}'", a.name.family_name().unwrap_or_else(|| "?".into()), b.name.family_name().unwrap_or_else(|| "?".into())));
            }
            if a.name.subfamily_name() != b.name.subfamily_name() {
                diffs.push(format!("Subfamily: '{}' vs '{}'", a.name.subfamily_name().unwrap_or_else(|| "?".into()), b.name.subfamily_name().unwrap_or_else(|| "?".into())));
            }
            if a.maxp.num_glyphs != b.maxp.num_glyphs {
                diffs.push(format!("Glyph count: {} vs {}", a.maxp.num_glyphs, b.maxp.num_glyphs));
            }
            if a.head.units_per_em != b.head.units_per_em {
                diffs.push(format!("Units per em: {} vs {}", a.head.units_per_em, b.head.units_per_em));
            }

            if diffs.is_empty() {
                println!("{} and {} are structurally identical", font_a.display(), font_b.display());
            } else {
                println!("Differences between {} and {}:", font_a.display(), font_b.display());
                for d in &diffs {
                    println!("  - {}", d);
                }
            }
        }
        Commands::ToWoff2 { font, out } => {
            let bytes = std::fs::read(&font)?;
            let font_obj = fonttype::Font::read(&bytes)?;
            let mut table_data: Vec<(fonttype::Tag, Vec<u8>)> = Vec::new();
            // Reconstruct raw table list from parsed font
            let written = font_obj.write()?;
            let mut p = fonttype::parse::Parser::new(&written, 0);
            let _sfnt_version = p.u32()?;
            let num_tables = p.u16()?;
            let _ = p.u16()?;
            let _ = p.u16()?;
            let _ = p.u16()?;
            for _ in 0..num_tables {
                let tag = p.tag()?;
                let _checksum = p.u32()?;
                let offset = p.u32()? as usize;
                let length = p.u32()? as usize;
                table_data.push((tag, written[offset..offset + length].to_vec()));
            }
            let woff2 = fonttype::write_woff2(&table_data)?;
            std::fs::write(&out, woff2)?;
            println!("Converted {} -> {} ({} bytes)", font.display(), out.display(), std::fs::metadata(&out)?.len());
        }
        Commands::FromWoff2 { font, out } => {
            let bytes = std::fs::read(&font)?;
            let table_data = fonttype::read_woff2(&bytes)?;
            // Reconstruct sfnt font from tables
            let mut w = fonttype::write::Writer::new();
            w.write_u32(0x00010000); // TrueType
            let num_tables = table_data.len() as u16;
            let search_range = (1u16 << (num_tables as f32).log2().floor() as u16) * 16;
            let entry_selector = (num_tables as f32).log2().floor() as u16;
            let range_shift = num_tables * 16 - search_range;
            w.write_u16(num_tables);
            w.write_u16(search_range);
            w.write_u16(entry_selector);
            w.write_u16(range_shift);
            let header_size = 12 + num_tables as usize * 16;
            let mut offset = header_size as u32;
            let mut records = Vec::with_capacity(table_data.len());
            for (tag, data) in &table_data {
                let mut padded = data.clone();
                while padded.len() % 4 != 0 {
                    padded.push(0);
                }
                let padded_len = padded.len() as u32;
                let checksum = fonttype::parse::checksum_table(&padded);
                records.push((tag.clone(), checksum, offset, data.len() as u32, padded));
                offset += padded_len;
            }
            for (tag, checksum, off, len, _) in &records {
                w.write_tag(&tag.0);
                w.write_u32(*checksum);
                w.write_u32(*off);
                w.write_u32(*len);
            }
            for (_, _, _, _, padded) in &records {
                w.write_bytes(padded);
            }
            let raw = w.into_vec();
            // Try to parse and rewrite with correct checksums
            match fonttype::Font::read(&raw) {
                Ok(font_obj) => {
                    std::fs::write(&out, font_obj.write()?)?;
                }
                Err(_) => {
                    std::fs::write(&out, raw)?;
                }
            }
            println!("Converted {} -> {}", font.display(), out.display());
        }
        Commands::TtcInfo { file } => {
            let bytes = std::fs::read(&file)?;
            let ttc = fonttype::Ttc::parse(&bytes)?;
            println!("TrueType Collection: {}", file.display());
            println!("  Version: {}.{}", ttc.version >> 16, ttc.version & 0xFFFF);
            println!("  Fonts: {}", ttc.num_fonts);
            for (i, result) in ttc.fonts(&bytes).iter().enumerate() {
                match result {
                    Ok(font) => {
                        let family = font.name.family_name().unwrap_or_else(|| "?".into());
                        println!("  [{}] {} ({} glyphs, {} tables)", i, family, font.maxp.num_glyphs, font.tables.len());
                    }
                    Err(e) => {
                        println!("  [{}] Error: {}", i, e);
                    }
                }
            }
        }
    }
    Ok(())
}
