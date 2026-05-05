use crate::field::{Anchor, ImageField, TextField};
use crate::font::TH_SARABUN_FONT_DATA;
use crate::template::template_bytes;
use anyhow::{anyhow, Context, Result};
use lopdf::{Dictionary, Document, Object, Stream};
use std::collections::HashMap;
use std::io::{self, Cursor, Write};

pub(crate) const PAGE_WIDTH: f64 = 595.28;
pub(crate) const PAGE_HEIGHT: f64 = 841.89;

// ── Anchor coordinate system ──────────────────────────────────────────────────

fn anchor_to_xy(anchor: Anchor, dx: f64, dy: f64) -> (f64, f64) {
    match anchor {
        Anchor::TopLeft => (dx, -dy),
        Anchor::TopCenter => (PAGE_WIDTH / 2.0 + dx, -dy),
        Anchor::TopRight => (PAGE_WIDTH + dx, -dy),
        Anchor::BottomLeft => (dx, PAGE_HEIGHT - dy),
        Anchor::BottomCenter => (PAGE_WIDTH / 2.0 + dx, PAGE_HEIGHT - dy),
        Anchor::BottomRight => (PAGE_WIDTH + dx, PAGE_HEIGHT - dy),
        Anchor::Center => (PAGE_WIDTH / 2.0 + dx, PAGE_HEIGHT / 2.0 - dy),
        Anchor::Left => (dx, PAGE_HEIGHT / 2.0 - dy),
        Anchor::Right => (PAGE_WIDTH + dx, PAGE_HEIGHT / 2.0 - dy),
    }
}

// ── GID encoder ───────────────────────────────────────────────────────────────
//
// Identity-H encoding requires CID values to be glyph indices (GID), NOT
// Unicode codepoints. This encoder parses the font's cmap table so every
// character is mapped to its actual GID before being written into the PDF
// content stream.

struct GidEncoder {
    char_to_gid: HashMap<char, u16>,
    gid_to_width: HashMap<u16, u16>, // GID → advance width in font units
    units_per_em: u16,
}

impl GidEncoder {
    fn new(font_data: &[u8]) -> Result<Self> {
        let face =
            ttf_parser::Face::parse(font_data, 0).map_err(|e| anyhow!("parse TTF: {:?}", e))?;
        let units_per_em = face.units_per_em();

        let mut char_to_gid: HashMap<char, u16> = HashMap::new();
        let mut gid_to_width: HashMap<u16, u16> = HashMap::new();

        // Collect all characters we might need: printable ASCII + Thai + common punctuation
        let ranges: &[(u32, u32)] = &[
            (0x0020, 0x007E), // printable ASCII
            (0x00A0, 0x00FF), // Latin-1 supplement
            (0x0E00, 0x0E7F), // Thai block
            (0x2000, 0x206F), // General punctuation
            (0x25A0, 0x25FF), // Geometric shapes (checkmarks etc)
        ];

        for &(start, end) in ranges {
            for cp in start..=end {
                if let Some(c) = char::from_u32(cp) {
                    if let Some(gid) = face.glyph_index(c) {
                        char_to_gid.insert(c, gid.0);
                        if let Some(adv) = face.glyph_hor_advance(gid) {
                            gid_to_width.insert(gid.0, adv);
                        }
                    }
                }
            }
        }

        Ok(Self {
            char_to_gid,
            gid_to_width,
            units_per_em,
        })
    }

    /// Encode text as big-endian GID bytes for Identity-H / CIDFontType2.
    fn encode(&self, text: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(text.len() * 2);
        for c in text.chars() {
            let gid = self.char_to_gid.get(&c).copied().unwrap_or(0);
            out.push((gid >> 8) as u8);
            out.push((gid & 0xFF) as u8);
        }
        out
    }

    /// Exact text width in PDF points at the given font size.
    fn text_width(&self, text: &str, font_size: f64) -> f64 {
        let advance: u32 = text
            .chars()
            .map(|c| {
                let gid = self.char_to_gid.get(&c).copied().unwrap_or(0);
                self.gid_to_width.get(&gid).copied().unwrap_or(500) as u32
            })
            .sum();
        advance as f64 * font_size / self.units_per_em as f64
    }

    /// Build the PDF W (widths) array for the CIDFont dictionary.
    /// Format: [ gid [width_in_glyph_units] ... ]
    fn w_array(&self) -> Vec<Object> {
        let scale = 1000.0 / self.units_per_em as f64;
        let mut pairs: Vec<(u16, u16)> = self.gid_to_width.iter().map(|(&g, &w)| (g, w)).collect();
        pairs.sort_by_key(|&(g, _)| g);

        let mut arr = Vec::with_capacity(pairs.len() * 2);
        for (gid, adv) in pairs {
            arr.push(Object::Integer(gid as i64));
            arr.push(Object::Array(vec![Object::Integer(
                (adv as f64 * scale).round() as i64,
            )]));
        }
        arr
    }

    /// Build a proper ToUnicode CMap: GID → Unicode character.
    fn to_unicode_cmap(&self) -> Vec<u8> {
        let mut gid_char: Vec<(u16, char)> =
            self.char_to_gid.iter().map(|(&c, &g)| (g, c)).collect();
        gid_char.sort_by_key(|&(g, _)| g);

        let mut buf: Vec<u8> = Vec::new();
        writeln!(buf, "/CIDInit /ProcSet findresource begin").ok();
        write!(buf, "12 dict begin\nbegincmap\n").ok();
        writeln!(
            buf,
            "/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def"
        )
        .ok();
        write!(buf, "/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n").ok();
        write!(
            buf,
            "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
        )
        .ok();

        for chunk in gid_char.chunks(100) {
            writeln!(buf, "{} beginbfchar", chunk.len()).ok();
            for &(gid, c) in chunk {
                let u16s: Vec<u16> = c.encode_utf16(&mut [0u16; 2]).to_vec();
                if u16s.len() == 1 {
                    writeln!(buf, "<{:04X}> <{:04X}>", gid, u16s[0]).ok();
                } else {
                    writeln!(buf, "<{:04X}> <{:04X}{:04X}>", gid, u16s[0], u16s[1]).ok();
                }
            }
            writeln!(buf, "endbfchar").ok();
        }

        write!(
            buf,
            "endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
        )
        .ok();
        buf
    }
}

// ── Main entry point ──────────────────────────────────────────────────────────

pub(crate) fn fill_certificate<W: io::Write>(
    out: &mut W,
    text_fields: Vec<TextField>,
    image_fields: Vec<ImageField>,
) -> Result<()> {
    // Build GID encoder from the embedded font
    let encoder = GidEncoder::new(TH_SARABUN_FONT_DATA).context("build GID encoder")?;

    // Load template and import page 1 as Form XObject into a fresh document
    let tpl_bytes = template_bytes();
    let mut tpl_doc = Document::load_from(Cursor::new(tpl_bytes)).context("load template PDF")?;
    tpl_doc.decompress();

    let mut doc = Document::with_version("1.7");
    let mut id_map: HashMap<lopdf::ObjectId, lopdf::ObjectId> = HashMap::new();

    let (xobj_id, media_box) =
        import_page_as_xobject(&tpl_doc, &mut doc, &mut id_map).context("import template page")?;

    // Embed Thai font with proper GID-based encoding
    let (font_id, font_name) = add_ttf_font(&mut doc, &encoder)?;

    // Build content stream
    let mut content: Vec<u8> = Vec::new();
    write!(content, "q\n/TplPage Do\nQ\n").ok(); // render template as background

    // Background images
    let mut img_xobjs: Vec<(String, lopdf::ObjectId)> = Vec::new();
    for img_field in &image_fields {
        if !img_field.on_top {
            if let Some(pair) = write_image_ops(&mut doc, &mut content, img_field) {
                img_xobjs.push(pair);
            }
        }
    }

    write_text_ops(&mut content, &text_fields, &font_name, &encoder);

    // Foreground images
    for img_field in &image_fields {
        if img_field.on_top {
            if let Some(pair) = write_image_ops(&mut doc, &mut content, img_field) {
                img_xobjs.push(pair);
            }
        }
    }

    // Assemble page
    let content_id = doc.add_object(Stream::new(Dictionary::new(), content));

    let mut xobjects = Dictionary::new();
    xobjects.set(b"TplPage".to_vec(), Object::Reference(xobj_id));
    for (name, id) in img_xobjs {
        xobjects.set(name.into_bytes(), Object::Reference(id));
    }

    let mut fonts = Dictionary::new();
    fonts.set(font_name.as_bytes(), Object::Reference(font_id));

    let mut resources = Dictionary::new();
    resources.set(b"XObject".to_vec(), Object::Dictionary(xobjects));
    resources.set(b"Font".to_vec(), Object::Dictionary(fonts));

    let mut page_dict = Dictionary::new();
    page_dict.set(b"Type".to_vec(), Object::Name(b"Page".to_vec()));
    page_dict.set(b"MediaBox".to_vec(), media_box);
    page_dict.set(b"Resources".to_vec(), Object::Dictionary(resources));
    page_dict.set(b"Contents".to_vec(), Object::Reference(content_id));
    let page_id = doc.add_object(Object::Dictionary(page_dict));

    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type".to_vec(), Object::Name(b"Pages".to_vec()));
    pages_dict.set(b"Count".to_vec(), Object::Integer(1));
    pages_dict.set(
        b"Kids".to_vec(),
        Object::Array(vec![Object::Reference(page_id)]),
    );
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));

    if let Ok(Object::Dictionary(d)) = doc.get_object_mut(page_id) {
        d.set(b"Parent".to_vec(), Object::Reference(pages_id));
    }

    let mut catalog = Dictionary::new();
    catalog.set(b"Type".to_vec(), Object::Name(b"Catalog".to_vec()));
    catalog.set(b"Pages".to_vec(), Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer
        .set(b"Root".to_vec(), Object::Reference(catalog_id));

    let mut buf = Vec::new();
    doc.save_to(&mut buf).context("save PDF")?;
    out.write_all(&buf).context("write output")?;
    Ok(())
}

// ── Template import (Form XObject) ────────────────────────────────────────────

fn import_page_as_xobject(
    tpl_doc: &Document,
    doc: &mut Document,
    id_map: &mut HashMap<lopdf::ObjectId, lopdf::ObjectId>,
) -> Result<(lopdf::ObjectId, Object)> {
    let pages = tpl_doc.get_pages();
    let page_id = *pages
        .get(&1)
        .ok_or_else(|| anyhow!("template has no page 1"))?;

    let page_dict = tpl_doc
        .get_object(page_id)?
        .as_dict()
        .context("page not a dict")?
        .clone();

    let media_box = page_dict
        .get(b"MediaBox")
        .or_else(|_| page_dict.get(b"CropBox"))
        .context("no MediaBox")?
        .clone();

    let resources_obj = match page_dict.get(b"Resources") {
        Ok(r) => deep_copy_object(tpl_doc, doc, id_map, r.clone())?,
        Err(_) => Object::Dictionary(Dictionary::new()),
    };

    let content_bytes = collect_content_streams(tpl_doc, &page_dict)?;

    let mut xobj_dict = Dictionary::new();
    xobj_dict.set(b"Type".to_vec(), Object::Name(b"XObject".to_vec()));
    xobj_dict.set(b"Subtype".to_vec(), Object::Name(b"Form".to_vec()));
    xobj_dict.set(b"BBox".to_vec(), media_box.clone());
    xobj_dict.set(b"Resources".to_vec(), resources_obj);

    let xobj_id = doc.add_object(Stream::new(xobj_dict, content_bytes));
    Ok((xobj_id, media_box))
}

fn collect_content_streams(doc: &Document, page_dict: &Dictionary) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let contents = match page_dict.get(b"Contents") {
        Ok(c) => c.clone(),
        Err(_) => return Ok(out),
    };

    let ids: Vec<lopdf::ObjectId> = match &contents {
        Object::Reference(id) => vec![*id],
        Object::Array(arr) => arr
            .iter()
            .filter_map(|o| {
                if let Object::Reference(id) = o {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    };

    for id in ids {
        if let Ok(obj) = doc.get_object(id) {
            if let Ok(stream) = obj.as_stream() {
                out.extend_from_slice(&stream.content);
                if out.last() != Some(&b'\n') {
                    out.push(b'\n');
                }
            }
        }
    }
    Ok(out)
}

fn deep_copy_object(
    src: &Document,
    dst: &mut Document,
    id_map: &mut HashMap<lopdf::ObjectId, lopdf::ObjectId>,
    obj: Object,
) -> Result<Object> {
    match obj {
        Object::Reference(src_id) => {
            if let Some(&dst_id) = id_map.get(&src_id) {
                return Ok(Object::Reference(dst_id));
            }
            let dst_id = dst.add_object(Object::Null);
            id_map.insert(src_id, dst_id);

            let src_obj = src
                .get_object(src_id).cloned()
                .unwrap_or(Object::Null);
            let copied = deep_copy_object(src, dst, id_map, src_obj)?;
            dst.objects.insert(dst_id, copied);
            Ok(Object::Reference(dst_id))
        }
        Object::Dictionary(dict) => {
            let mut new_dict = Dictionary::new();
            for (k, v) in dict.iter() {
                new_dict.set(k.clone(), deep_copy_object(src, dst, id_map, v.clone())?);
            }
            Ok(Object::Dictionary(new_dict))
        }
        Object::Array(arr) => {
            let mut new_arr = Vec::with_capacity(arr.len());
            for v in arr {
                new_arr.push(deep_copy_object(src, dst, id_map, v)?);
            }
            Ok(Object::Array(new_arr))
        }
        Object::Stream(s) => {
            let mut new_dict = Dictionary::new();
            for (k, v) in s.dict.iter() {
                if k == b"Filter" || k == b"DecodeParms" {
                    continue;
                }
                new_dict.set(k.clone(), deep_copy_object(src, dst, id_map, v.clone())?);
            }
            Ok(Object::Stream(Stream::new(new_dict, s.content.clone())))
        }
        other => Ok(other),
    }
}

// ── Font embedding ────────────────────────────────────────────────────────────

fn add_ttf_font(doc: &mut Document, encoder: &GidEncoder) -> Result<(lopdf::ObjectId, String)> {
    let font_res_name = "ThSrbn";
    let font_data = TH_SARABUN_FONT_DATA;
    let compressed = compress_zlib(font_data);

    let mut fs_dict = Dictionary::new();
    fs_dict.set(b"Length".to_vec(), Object::Integer(compressed.len() as i64));
    fs_dict.set(b"Length1".to_vec(), Object::Integer(font_data.len() as i64));
    fs_dict.set(b"Filter".to_vec(), Object::Name(b"FlateDecode".to_vec()));
    let ff_id = doc.add_object(Stream::new(fs_dict, compressed));

    let mut desc = Dictionary::new();
    desc.set(b"Type".to_vec(), Object::Name(b"FontDescriptor".to_vec()));
    desc.set(b"FontName".to_vec(), Object::Name(b"THSarabunNew".to_vec()));
    desc.set(b"Flags".to_vec(), Object::Integer(32));
    desc.set(b"ItalicAngle".to_vec(), Object::Integer(0));
    desc.set(b"Ascent".to_vec(), Object::Integer(800));
    desc.set(b"Descent".to_vec(), Object::Integer(-200));
    desc.set(b"CapHeight".to_vec(), Object::Integer(700));
    desc.set(b"StemV".to_vec(), Object::Integer(80));
    desc.set(
        b"FontBBox".to_vec(),
        Object::Array(vec![
            Object::Integer(-300),
            Object::Integer(-300),
            Object::Integer(1200),
            Object::Integer(1000),
        ]),
    );
    desc.set(b"FontFile2".to_vec(), Object::Reference(ff_id));
    let desc_id = doc.add_object(Object::Dictionary(desc));

    // CIDFont with proper W (widths) array built from actual font metrics
    let mut cid = Dictionary::new();
    cid.set(b"Type".to_vec(), Object::Name(b"Font".to_vec()));
    cid.set(b"Subtype".to_vec(), Object::Name(b"CIDFontType2".to_vec()));
    cid.set(b"BaseFont".to_vec(), Object::Name(b"THSarabunNew".to_vec()));
    cid.set(
        b"CIDSystemInfo".to_vec(),
        Object::Dictionary({
            let mut d = Dictionary::new();
            d.set(b"Registry".to_vec(), Object::string_literal("Adobe"));
            d.set(b"Ordering".to_vec(), Object::string_literal("Identity"));
            d.set(b"Supplement".to_vec(), Object::Integer(0));
            d
        }),
    );
    cid.set(b"FontDescriptor".to_vec(), Object::Reference(desc_id));
    cid.set(b"DW".to_vec(), Object::Integer(1000));
    cid.set(b"W".to_vec(), Object::Array(encoder.w_array())); // exact per-glyph widths
    let cid_id = doc.add_object(Object::Dictionary(cid));

    // Proper ToUnicode CMap (GID → Unicode)
    let cmap_bytes = encoder.to_unicode_cmap();
    let mut cmap_dict = Dictionary::new();
    cmap_dict.set(b"Length".to_vec(), Object::Integer(cmap_bytes.len() as i64));
    let cmap_id = doc.add_object(Stream::new(cmap_dict, cmap_bytes));

    let mut font = Dictionary::new();
    font.set(b"Type".to_vec(), Object::Name(b"Font".to_vec()));
    font.set(b"Subtype".to_vec(), Object::Name(b"Type0".to_vec()));
    font.set(b"BaseFont".to_vec(), Object::Name(b"THSarabunNew".to_vec()));
    font.set(b"Encoding".to_vec(), Object::Name(b"Identity-H".to_vec()));
    font.set(
        b"DescendantFonts".to_vec(),
        Object::Array(vec![Object::Reference(cid_id)]),
    );
    font.set(b"ToUnicode".to_vec(), Object::Reference(cmap_id));
    let font_id = doc.add_object(Object::Dictionary(font));

    Ok((font_id, font_res_name.to_string()))
}

// ── Text placement ────────────────────────────────────────────────────────────

fn write_text_ops(buf: &mut Vec<u8>, fields: &[TextField], font_name: &str, enc: &GidEncoder) {
    for field in fields {
        if field.text == "✓" {
            let (x, y_td) = anchor_to_xy(field.position, field.dx, field.dy);
            write_checkmark_ops(buf, x, PAGE_HEIGHT - y_td, field.font_size as f64);
        } else {
            let (x, y_td) = anchor_to_xy(field.position, field.dx, field.dy);
            let fs = field.font_size as f64;
            let y_pdf = PAGE_HEIGHT - y_td;

            // Use exact GID-based text width for alignment
            let text_w = enc.text_width(&field.text, fs);
            let x_adj = match field.position {
                Anchor::TopCenter | Anchor::BottomCenter | Anchor::Center => x - text_w / 2.0,
                Anchor::TopRight | Anchor::BottomRight | Anchor::Right => x - text_w,
                _ => x,
            };

            // Encode with actual GIDs (not raw Unicode codepoints)
            let gid_bytes = enc.encode(&field.text);

            write!(
                buf,
                "BT\n/{} {} Tf\n{:.4} {:.4} Td\n<",
                font_name, field.font_size, x_adj, y_pdf
            )
            .ok();
            for byte in &gid_bytes {
                write!(buf, "{:02X}", byte).ok();
            }
            write!(buf, "> Tj\nET\n").ok();
        }
    }
}

// ── Checkmark ─────────────────────────────────────────────────────────────────

fn write_checkmark_ops(buf: &mut Vec<u8>, x: f64, y_pdf_anchor: f64, size: f64) {
    const N: usize = 28;
    const C: usize = 16;
    let h = size * 0.125;

    let p0rx = 0.10 * size;
    let p0ry = -0.42 * size;
    let p1rx = 0.27 * size;
    let p1ry = -0.77 * size;
    let c1rx = 0.44 * size;
    let c1ry = -0.83 * size;
    let c2rx = 0.80 * size;
    let c2ry = -0.23 * size;
    let p2rx = 0.97 * size;
    let p2ry = -0.07 * size;

    let ldx = p1rx - p0rx;
    let ldy = p1ry - p0ry;
    let ll = (ldx * ldx + ldy * ldy).sqrt();
    let (ldx, ldy) = (ldx / ll, ldy / ll);
    let (lox, loy) = (-ldy, ldx);
    let (lix, liy) = (ldy, -ldx);
    let (lbx, lby) = (-ldx, -ldy);

    let rtdx = p2rx - c2rx;
    let rtdy = p2ry - c2ry;
    let rl = (rtdx * rtdx + rtdy * rtdy).sqrt();
    let (rtdx, rtdy) = (rtdx / rl, rtdy / rl);
    let (rox, roy) = (-rtdy, rtdx);

    let rvtdx = c1rx - p1rx;
    let rvtdy = c1ry - p1ry;
    let rvl = (rvtdx * rvtdx + rvtdy * rvtdy).sqrt();
    let (rvtdx, rvtdy) = (rvtdx / rvl, rvtdy / rvl);

    let bez_oi = |t: f64, hh: f64| -> ((f64, f64), (f64, f64)) {
        let u = 1.0 - t;
        let px =
            u * u * u * p1rx + 3.0 * u * u * t * c1rx + 3.0 * u * t * t * c2rx + t * t * t * p2rx;
        let py =
            u * u * u * p1ry + 3.0 * u * u * t * c1ry + 3.0 * u * t * t * c2ry + t * t * t * p2ry;
        let tdx =
            3.0 * (u * u * (c1rx - p1rx) + 2.0 * u * t * (c2rx - c1rx) + t * t * (p2rx - c2rx));
        let tdy =
            3.0 * (u * u * (c1ry - p1ry) + 2.0 * u * t * (c2ry - c1ry) + t * t * (p2ry - c2ry));
        let tl = (tdx * tdx + tdy * tdy).sqrt();
        let (tdx, tdy) = (tdx / tl, tdy / tl);
        (
            (px - tdy * hh, py + tdx * hh),
            (px + tdy * hh, py - tdx * hh),
        )
    };

    let mut poly: Vec<(f64, f64)> = Vec::with_capacity(2 * N + 2 * C + 40);

    for i in 0..=N {
        let t = i as f64 / N as f64;
        poly.push((
            p0rx + t * (p1rx - p0rx) + lox * h,
            p0ry + t * (p1ry - p0ry) + loy * h,
        ));
    }
    poly.push((p1rx, p1ry - h));
    for i in 1..=N {
        let (o, _) = bez_oi(i as f64 / N as f64, h);
        poly.push(o);
    }
    for i in 0..=C {
        let a = std::f64::consts::PI * i as f64 / C as f64;
        let (c, s) = (a.cos(), a.sin());
        poly.push((
            p2rx + (c * rox + s * rtdx) * h,
            p2ry + (c * roy + s * rtdy) * h,
        ));
    }
    for i in (1..N).rev() {
        let (_, pi) = bez_oi(i as f64 / N as f64, h);
        poly.push(pi);
    }

    let riv_x = p1rx + rvtdy * h;
    let riv_y = p1ry - rvtdx * h;
    let liv_x = p1rx + lix * h;
    let liv_y = p1ry + liy * h;
    let ct_x = (riv_x + liv_x) * 0.5;
    let ct_y = p1ry - h * 0.6;
    for i in 0..=8 {
        let t = i as f64 / 8.0;
        let u = 1.0 - t;
        poly.push((
            u * u * riv_x + 2.0 * u * t * ct_x + t * t * liv_x,
            u * u * riv_y + 2.0 * u * t * ct_y + t * t * liv_y,
        ));
    }
    for i in (0..N).rev() {
        let t = i as f64 / N as f64;
        poly.push((
            p0rx + t * (p1rx - p0rx) + lix * h,
            p0ry + t * (p1ry - p0ry) + liy * h,
        ));
    }
    for i in 0..=C {
        let a = std::f64::consts::PI * i as f64 / C as f64;
        let (c, s) = (a.cos(), a.sin());
        poly.push((
            p0rx + (c * lix + s * lbx) * h,
            p0ry + (c * liy + s * lby) * h,
        ));
    }

    writeln!(buf, "0 0 0 rg").ok();
    if let Some(&(fx, fy)) = poly.first() {
        writeln!(buf, "{:.4} {:.4} m", x + fx, y_pdf_anchor + fy).ok();
        for &(px, py) in poly.iter().skip(1) {
            writeln!(buf, "{:.4} {:.4} l", x + px, y_pdf_anchor + py).ok();
        }
        writeln!(buf, "h f").ok();
    }
}

// ── Image placement ───────────────────────────────────────────────────────────

fn write_image_ops(
    doc: &mut Document,
    buf: &mut Vec<u8>,
    img_field: &ImageField,
) -> Option<(String, lopdf::ObjectId)> {
    if img_field.data.is_empty() {
        return None;
    }

    let dyn_img = image::load_from_memory(&img_field.data).ok()?;
    let (iw, ih) = (dyn_img.width(), dyn_img.height());
    if iw == 0 || ih == 0 || (iw == 1 && ih == 1) {
        return None;
    }

    let disp_w = PAGE_WIDTH * img_field.scale;
    let disp_h = disp_w * ih as f64 / iw as f64;

    let (x, y_td) = anchor_to_xy(img_field.pos, img_field.dx, img_field.dy);
    let y_pdf = PAGE_HEIGHT - y_td - disp_h;

    let rgba = dyn_img.to_rgba8();
    let rgb: Vec<u8> = rgba.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
    let alpha: Vec<u8> = rgba.pixels().map(|p| p[3]).collect();

    let c_rgb = compress_zlib(&rgb);
    let c_alpha = compress_zlib(&alpha);

    let mut sm = Dictionary::new();
    sm.set(b"Type".to_vec(), Object::Name(b"XObject".to_vec()));
    sm.set(b"Subtype".to_vec(), Object::Name(b"Image".to_vec()));
    sm.set(b"Width".to_vec(), Object::Integer(iw as i64));
    sm.set(b"Height".to_vec(), Object::Integer(ih as i64));
    sm.set(b"ColorSpace".to_vec(), Object::Name(b"DeviceGray".to_vec()));
    sm.set(b"BitsPerComponent".to_vec(), Object::Integer(8));
    sm.set(b"Filter".to_vec(), Object::Name(b"FlateDecode".to_vec()));
    sm.set(b"Length".to_vec(), Object::Integer(c_alpha.len() as i64));
    let smask_id = doc.add_object(Stream::new(sm, c_alpha));

    let xobj_name = format!("Img{}", doc.objects.len());
    let mut im = Dictionary::new();
    im.set(b"Type".to_vec(), Object::Name(b"XObject".to_vec()));
    im.set(b"Subtype".to_vec(), Object::Name(b"Image".to_vec()));
    im.set(b"Width".to_vec(), Object::Integer(iw as i64));
    im.set(b"Height".to_vec(), Object::Integer(ih as i64));
    im.set(b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec()));
    im.set(b"BitsPerComponent".to_vec(), Object::Integer(8));
    im.set(b"Filter".to_vec(), Object::Name(b"FlateDecode".to_vec()));
    im.set(b"Length".to_vec(), Object::Integer(c_rgb.len() as i64));
    im.set(b"SMask".to_vec(), Object::Reference(smask_id));
    let img_id = doc.add_object(Stream::new(im, c_rgb));

    write!(
        buf,
        "q\n{:.4} 0 0 {:.4} {:.4} {:.4} cm\n/{} Do\nQ\n",
        disp_w, disp_h, x, y_pdf, xobj_name
    )
    .ok();

    Some((xobj_name, img_id))
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn compress_zlib(data: &[u8]) -> Vec<u8> {
    use flate2::{write::ZlibEncoder, Compression};
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_top_left_applies_dx_dy_directly() {
        let (x, y) = anchor_to_xy(Anchor::TopLeft, 10.0, 20.0);
        assert_eq!(x, 10.0);
        assert_eq!(y, -20.0);
    }

    #[test]
    fn anchor_top_center_centers_horizontally() {
        let (x, y) = anchor_to_xy(Anchor::TopCenter, 0.0, 0.0);
        assert!((x - PAGE_WIDTH / 2.0).abs() < 0.01);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn anchor_top_right_starts_at_page_right() {
        let (x, y) = anchor_to_xy(Anchor::TopRight, 0.0, 0.0);
        assert!((x - PAGE_WIDTH).abs() < 0.01);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn anchor_left_centers_vertically() {
        let (x, y) = anchor_to_xy(Anchor::Left, 0.0, 0.0);
        assert_eq!(x, 0.0);
        assert!((y - PAGE_HEIGHT / 2.0).abs() < 0.01);
    }

    #[test]
    fn anchor_center_centers_both_axes() {
        let (x, y) = anchor_to_xy(Anchor::Center, 0.0, 0.0);
        assert!((x - PAGE_WIDTH / 2.0).abs() < 0.01);
        assert!((y - PAGE_HEIGHT / 2.0).abs() < 0.01);
    }

    #[test]
    fn anchor_right_starts_at_page_right() {
        let (x, y) = anchor_to_xy(Anchor::Right, 0.0, 0.0);
        assert!((x - PAGE_WIDTH).abs() < 0.01);
        assert!((y - PAGE_HEIGHT / 2.0).abs() < 0.01);
    }

    #[test]
    fn anchor_bottom_left_starts_at_page_bottom() {
        let (x, y) = anchor_to_xy(Anchor::BottomLeft, 0.0, 0.0);
        assert_eq!(x, 0.0);
        assert!((y - PAGE_HEIGHT).abs() < 0.01);
    }

    #[test]
    fn anchor_bottom_center_centers_horizontally_at_bottom() {
        let (x, y) = anchor_to_xy(Anchor::BottomCenter, 0.0, 100.0);
        assert!((x - PAGE_WIDTH / 2.0).abs() < 0.01);
        assert!((y - (PAGE_HEIGHT - 100.0)).abs() < 0.01);
    }

    #[test]
    fn anchor_bottom_right_starts_at_page_right_bottom() {
        let (x, y) = anchor_to_xy(Anchor::BottomRight, 0.0, 0.0);
        assert!((x - PAGE_WIDTH).abs() < 0.01);
        assert!((y - PAGE_HEIGHT).abs() < 0.01);
    }

    #[test]
    fn anchor_offsets_are_additive() {
        let (x, y) = anchor_to_xy(Anchor::Center, 10.0, 5.0);
        assert!((x - (PAGE_WIDTH / 2.0 + 10.0)).abs() < 0.01);
        assert!((y - (PAGE_HEIGHT / 2.0 - 5.0)).abs() < 0.01);
    }
}
