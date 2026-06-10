use std::path::PathBuf;
use clap::{Parser, Subcommand};

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
    }
    Ok(())
}
