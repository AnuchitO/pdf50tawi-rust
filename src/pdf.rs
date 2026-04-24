use std::collections::HashMap;
use std::io::{self, Write, Cursor};
use anyhow::{Context, Result, anyhow};
use lopdf::{Document, Object, Stream, Dictionary};
use crate::field::{TextField, ImageField, Anchor};
use crate::font::TH_SARABUN_FONT_DATA;
use crate::template::template_bytes;

pub const PAGE_WIDTH: f64 = 595.28;
pub const PAGE_HEIGHT: f64 = 841.89;

/// Converts anchor + offset to absolute PDF coordinates (origin = top-left, y-down).
pub fn anchor_to_xy(anchor: Anchor, dx: f64, dy: f64) -> (f64, f64) {
    match anchor {
        Anchor::TopLeft     => (dx, -dy),
        Anchor::TopCenter   => (PAGE_WIDTH / 2.0 + dx, -dy),
        Anchor::TopRight    => (PAGE_WIDTH + dx, -dy),
        Anchor::BottomLeft  => (dx, PAGE_HEIGHT - dy),
        Anchor::BottomCenter=> (PAGE_WIDTH / 2.0 + dx, PAGE_HEIGHT - dy),
        Anchor::BottomRight => (PAGE_WIDTH + dx, PAGE_HEIGHT - dy),
        Anchor::Center      => (PAGE_WIDTH / 2.0 + dx, PAGE_HEIGHT / 2.0 - dy),
        Anchor::Left        => (dx, PAGE_HEIGHT / 2.0 - dy),
        Anchor::Right       => (PAGE_WIDTH + dx, PAGE_HEIGHT / 2.0 - dy),
    }
}

/// fill_certificate builds the output PDF:
///   1. Import template page 1 as a Form XObject into a fresh document.
///   2. Embed THSarabunNew TTF font.
///   3. Render the XObject, then overlay text and images.
///
/// This mirrors the Go approach (gopdf + gofpdi) exactly: a clean new document
/// with the template embedded as an XObject, so no existing resources are touched.
pub fn fill_certificate<W: io::Write>(
    text_fields: Vec<TextField>,
    image_fields: Vec<ImageField>,
    out: &mut W,
) -> Result<()> {
    // ── 1. Load template and import page 1 as Form XObject ───────────────────
    let tpl_bytes = template_bytes();
    let mut tpl_doc = Document::load_from(Cursor::new(tpl_bytes))
        .context("load template PDF")?;
    tpl_doc.decompress();  // ensure all streams are in raw (uncompressed) state

    let mut doc = Document::with_version("1.7");

    // id_map: tpl object id → new doc object id (to break reference cycles)
    let mut id_map: HashMap<lopdf::ObjectId, lopdf::ObjectId> = HashMap::new();

    let (xobj_id, media_box) = import_page_as_xobject(&tpl_doc, &mut doc, &mut id_map)
        .context("import template page")?;

    // ── 2. Add Thai TTF font ──────────────────────────────────────────────────
    let (font_id, font_name) = add_ttf_font(&mut doc)?;

    // ── 3. Build content stream ───────────────────────────────────────────────
    let mut content: Vec<u8> = Vec::new();

    // Render template as background
    write!(content, "q\n/TplPage Do\nQ\n").ok();

    // Background images (on_top = false)
    let mut img_xobjs: Vec<(String, lopdf::ObjectId)> = Vec::new();
    for img_field in &image_fields {
        if !img_field.on_top {
            if let Some((name, id)) = write_image_ops(&mut doc, &mut content, img_field) {
                img_xobjs.push((name, id));
            }
        }
    }

    // Text overlay
    write_text_ops(&mut content, &text_fields, &font_name);

    // Foreground images (on_top = true)
    for img_field in &image_fields {
        if img_field.on_top {
            if let Some((name, id)) = write_image_ops(&mut doc, &mut content, img_field) {
                img_xobjs.push((name, id));
            }
        }
    }

    // ── 4. Assemble page ──────────────────────────────────────────────────────
    let content_id = doc.add_object(Stream::new(Dictionary::new(), content));

    // XObject resources: template + images
    let mut xobjects = Dictionary::new();
    xobjects.set(b"TplPage".to_vec(), Object::Reference(xobj_id));
    for (name, id) in img_xobjs {
        xobjects.set(name.into_bytes(), Object::Reference(id));
    }

    // Font resources
    let mut fonts = Dictionary::new();
    fonts.set(font_name.as_bytes(), Object::Reference(font_id));

    let mut resources = Dictionary::new();
    resources.set(b"XObject".to_vec(), Object::Dictionary(xobjects));
    resources.set(b"Font".to_vec(), Object::Dictionary(fonts));

    let mut page_dict = Dictionary::new();
    page_dict.set(b"Type".to_vec(),      Object::Name(b"Page".to_vec()));
    page_dict.set(b"MediaBox".to_vec(),  media_box);
    page_dict.set(b"Resources".to_vec(), Object::Dictionary(resources));
    page_dict.set(b"Contents".to_vec(),  Object::Reference(content_id));

    let page_id = doc.add_object(Object::Dictionary(page_dict));

    // Pages node
    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type".to_vec(),  Object::Name(b"Pages".to_vec()));
    pages_dict.set(b"Count".to_vec(), Object::Integer(1));
    pages_dict.set(b"Kids".to_vec(),  Object::Array(vec![Object::Reference(page_id)]));
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));

    // Back-fill Parent on page
    if let Ok(Object::Dictionary(d)) = doc.get_object_mut(page_id) {
        d.set(b"Parent".to_vec(), Object::Reference(pages_id));
    }

    // Catalog
    let mut catalog = Dictionary::new();
    catalog.set(b"Type".to_vec(),  Object::Name(b"Catalog".to_vec()));
    catalog.set(b"Pages".to_vec(), Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));

    doc.trailer.set(b"Root".to_vec(), Object::Reference(catalog_id));

    // ── 5. Write output ───────────────────────────────────────────────────────
    let mut buf = Vec::new();
    doc.save_to(&mut buf).context("save PDF")?;
    out.write_all(&buf).context("write output")?;
    Ok(())
}

// ── Template import ───────────────────────────────────────────────────────────

/// Import page 1 of tpl_doc into doc as a Form XObject.
/// Returns (xobj_id, MediaBox).
fn import_page_as_xobject(
    tpl_doc: &Document,
    doc: &mut Document,
    id_map: &mut HashMap<lopdf::ObjectId, lopdf::ObjectId>,
) -> Result<(lopdf::ObjectId, Object)> {
    let pages = tpl_doc.get_pages();
    let page_id = *pages.get(&1).ok_or_else(|| anyhow!("template has no page 1"))?;

    let page_dict = tpl_doc.get_object(page_id)
        .context("get template page")?
        .as_dict()
        .context("page is not a dict")?
        .clone();

    // Resolve inherited MediaBox
    let media_box = page_dict.get(b"MediaBox")
        .or_else(|_| page_dict.get(b"CropBox"))
        .context("no MediaBox in template")?
        .clone();

    // Deep-copy Resources from tpl_doc → doc
    let resources_obj = match page_dict.get(b"Resources") {
        Ok(r) => deep_copy_object(tpl_doc, doc, id_map, r.clone())?,
        Err(_) => Object::Dictionary(Dictionary::new()),
    };

    // Collect and concatenate all content streams (already decompressed by tpl_doc.decompress())
    let content_bytes = collect_content_streams(tpl_doc, &page_dict)?;

    // Build Form XObject
    let mut xobj_dict = Dictionary::new();
    xobj_dict.set(b"Type".to_vec(),    Object::Name(b"XObject".to_vec()));
    xobj_dict.set(b"Subtype".to_vec(), Object::Name(b"Form".to_vec()));
    xobj_dict.set(b"BBox".to_vec(),    media_box.clone());
    xobj_dict.set(b"Resources".to_vec(), resources_obj);

    let xobj_id = doc.add_object(Stream::new(xobj_dict, content_bytes));
    Ok((xobj_id, media_box))
}

/// Collect and concatenate all content stream bytes for a page.
fn collect_content_streams(doc: &Document, page_dict: &Dictionary) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let contents = match page_dict.get(b"Contents") {
        Ok(c) => c.clone(),
        Err(_) => return Ok(out),
    };

    let ids: Vec<lopdf::ObjectId> = match &contents {
        Object::Reference(id) => vec![*id],
        Object::Array(arr) => arr.iter().filter_map(|o| {
            if let Object::Reference(id) = o { Some(*id) } else { None }
        }).collect(),
        _ => vec![],
    };

    for id in ids {
        if let Ok(obj) = doc.get_object(id) {
            if let Ok(stream) = obj.as_stream() {
                out.extend_from_slice(&stream.content);
                // ensure streams are separated by whitespace
                if out.last() != Some(&b'\n') {
                    out.push(b'\n');
                }
            }
        }
    }
    Ok(out)
}

/// Recursively deep-copy a PDF object from tpl_doc into doc, remapping all
/// indirect references via id_map to avoid duplication and cycles.
fn deep_copy_object(
    src: &Document,
    dst: &mut Document,
    id_map: &mut HashMap<lopdf::ObjectId, lopdf::ObjectId>,
    obj: Object,
) -> Result<Object> {
    match obj {
        Object::Reference(src_id) => {
            // Already mapped?
            if let Some(&dst_id) = id_map.get(&src_id) {
                return Ok(Object::Reference(dst_id));
            }
            // Reserve a new ID first (breaks cycles)
            let dst_id = dst.add_object(Object::Null);
            id_map.insert(src_id, dst_id);

            // Fetch and copy the referenced object
            let src_obj = match src.get_object(src_id) {
                Ok(o) => o.clone(),
                Err(_) => Object::Null,
            };
            let copied = deep_copy_object(src, dst, id_map, src_obj)?;
            // Replace the placeholder
            dst.objects.insert(dst_id, copied);
            Ok(Object::Reference(dst_id))
        }
        Object::Dictionary(dict) => {
            let mut new_dict = Dictionary::new();
            for (k, v) in dict.iter() {
                let new_v = deep_copy_object(src, dst, id_map, v.clone())?;
                new_dict.set(k.clone(), new_v);
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
                // Skip Filter/Length — content is already raw after decompress()
                if k == b"Filter" || k == b"DecodeParms" { continue; }
                let new_v = deep_copy_object(src, dst, id_map, v.clone())?;
                new_dict.set(k.clone(), new_v);
            }
            Ok(Object::Stream(Stream::new(new_dict, s.content.clone())))
        }
        other => Ok(other),
    }
}

// ── Font embedding ────────────────────────────────────────────────────────────

/// Embed THSarabunNew as a Type0/Identity-H font. Returns (font object id, resource name).
fn add_ttf_font(doc: &mut Document) -> Result<(lopdf::ObjectId, String)> {
    let font_res_name = "ThSrbn";
    let font_data = TH_SARABUN_FONT_DATA;
    let compressed = compress_zlib(font_data);

    // FontFile2
    let mut fs_dict = Dictionary::new();
    fs_dict.set(b"Length".to_vec(),  Object::Integer(compressed.len() as i64));
    fs_dict.set(b"Length1".to_vec(), Object::Integer(font_data.len() as i64));
    fs_dict.set(b"Filter".to_vec(),  Object::Name(b"FlateDecode".to_vec()));
    let ff_id = doc.add_object(Stream::new(fs_dict, compressed));

    // FontDescriptor
    let mut desc = Dictionary::new();
    desc.set(b"Type".to_vec(),      Object::Name(b"FontDescriptor".to_vec()));
    desc.set(b"FontName".to_vec(),  Object::Name(b"THSarabunNew".to_vec()));
    desc.set(b"Flags".to_vec(),     Object::Integer(32));
    desc.set(b"ItalicAngle".to_vec(), Object::Integer(0));
    desc.set(b"Ascent".to_vec(),    Object::Integer(800));
    desc.set(b"Descent".to_vec(),   Object::Integer(-200));
    desc.set(b"CapHeight".to_vec(), Object::Integer(700));
    desc.set(b"StemV".to_vec(),     Object::Integer(80));
    desc.set(b"FontBBox".to_vec(),  Object::Array(vec![
        Object::Integer(-300), Object::Integer(-300),
        Object::Integer(1200), Object::Integer(1000),
    ]));
    desc.set(b"FontFile2".to_vec(), Object::Reference(ff_id));
    let desc_id = doc.add_object(Object::Dictionary(desc));

    // CIDFont (Type2)
    let mut cid = Dictionary::new();
    cid.set(b"Type".to_vec(),    Object::Name(b"Font".to_vec()));
    cid.set(b"Subtype".to_vec(), Object::Name(b"CIDFontType2".to_vec()));
    cid.set(b"BaseFont".to_vec(),Object::Name(b"THSarabunNew".to_vec()));
    cid.set(b"CIDSystemInfo".to_vec(), Object::Dictionary({
        let mut d = Dictionary::new();
        d.set(b"Registry".to_vec(),   Object::string_literal("Adobe"));
        d.set(b"Ordering".to_vec(),   Object::string_literal("Identity"));
        d.set(b"Supplement".to_vec(), Object::Integer(0));
        d
    }));
    cid.set(b"FontDescriptor".to_vec(), Object::Reference(desc_id));
    cid.set(b"DW".to_vec(), Object::Integer(1000));
    let cid_id = doc.add_object(Object::Dictionary(cid));

    // ToUnicode CMap
    let cmap: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
        12 dict begin\nbegincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
        /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
        1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
        endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
    let mut cmap_dict = Dictionary::new();
    cmap_dict.set(b"Length".to_vec(), Object::Integer(cmap.len() as i64));
    let cmap_id = doc.add_object(Stream::new(cmap_dict, cmap.to_vec()));

    // Type0
    let mut font = Dictionary::new();
    font.set(b"Type".to_vec(),            Object::Name(b"Font".to_vec()));
    font.set(b"Subtype".to_vec(),         Object::Name(b"Type0".to_vec()));
    font.set(b"BaseFont".to_vec(),        Object::Name(b"THSarabunNew".to_vec()));
    font.set(b"Encoding".to_vec(),        Object::Name(b"Identity-H".to_vec()));
    font.set(b"DescendantFonts".to_vec(), Object::Array(vec![Object::Reference(cid_id)]));
    font.set(b"ToUnicode".to_vec(),       Object::Reference(cmap_id));
    let font_id = doc.add_object(Object::Dictionary(font));

    Ok((font_id, font_res_name.to_string()))
}

// ── Text placement ────────────────────────────────────────────────────────────

fn write_text_ops(buf: &mut Vec<u8>, fields: &[TextField], font_name: &str) {
    for field in fields {
        if field.text == "✓" {
            let (x, y_td) = anchor_to_xy(field.position, field.dx, field.dy);
            let y_pdf = PAGE_HEIGHT - y_td;
            write_checkmark_ops(buf, x, y_pdf, field.font_size as f64);
        } else {
            let (x, y_td) = anchor_to_xy(field.position, field.dx, field.dy);
            let fs = field.font_size as f64;
            let y_pdf = PAGE_HEIGHT - y_td - fs * 0.75;

            // UTF-16BE for Identity-H / Type0
            let utf16: Vec<u16> = field.text.encode_utf16().collect();
            let mut encoded = Vec::with_capacity(utf16.len() * 2);
            for code in &utf16 {
                encoded.push((code >> 8) as u8);
                encoded.push((code & 0xFF) as u8);
            }

            // Approximate width for alignment
            let char_count = field.text.chars().count() as f64;
            let approx_w = fs * 0.55 * char_count;
            let x_adj = match field.position {
                Anchor::TopCenter | Anchor::BottomCenter | Anchor::Center => x - approx_w / 2.0,
                Anchor::TopRight  | Anchor::BottomRight  | Anchor::Right  => x - approx_w,
                _ => x,
            };

            write!(buf, "BT\n/{} {} Tf\n{:.4} {:.4} Td\n<",
                   font_name, field.font_size, x_adj, y_pdf).ok();
            for byte in &encoded { write!(buf, "{:02X}", byte).ok(); }
            write!(buf, "> Tj\nET\n").ok();
        }
    }
}

// ── Checkmark ─────────────────────────────────────────────────────────────────

/// Exact port of Go's drawCheckmark: bold vector polygon, 28-sample arms, 16-step caps.
fn write_checkmark_ops(buf: &mut Vec<u8>, x: f64, y_pdf_anchor: f64, size: f64) {
    const N: usize = 28;
    const C: usize = 16;

    let h = size * 0.125;

    // Geometry in PDF-relative coords (x right, y up from anchor)
    let p0rx = 0.10 * size; let p0ry = -0.42 * size;
    let p1rx = 0.27 * size; let p1ry = -0.77 * size;
    let c1rx = 0.44 * size; let c1ry = -0.83 * size;
    let c2rx = 0.80 * size; let c2ry = -0.23 * size;
    let p2rx = 0.97 * size; let p2ry = -0.07 * size;

    let ldx = p1rx - p0rx; let ldy = p1ry - p0ry;
    let ll = (ldx*ldx + ldy*ldy).sqrt();
    let (ldx, ldy) = (ldx/ll, ldy/ll);
    let (lox, loy) = (-ldy, ldx);
    let (lix, liy) = (ldy, -ldx);
    let (lbx, lby) = (-ldx, -ldy);

    let rtdx = p2rx - c2rx; let rtdy = p2ry - c2ry;
    let rl = (rtdx*rtdx + rtdy*rtdy).sqrt();
    let (rtdx, rtdy) = (rtdx/rl, rtdy/rl);
    let (rox, roy) = (-rtdy, rtdx);

    let rvtdx = c1rx - p1rx; let rvtdy = c1ry - p1ry;
    let rvl = (rvtdx*rvtdx + rvtdy*rvtdy).sqrt();
    let (rvtdx, rvtdy) = (rvtdx/rvl, rvtdy/rvl);

    let bez_oi = |t: f64, hh: f64| -> ((f64,f64),(f64,f64)) {
        let u = 1.0 - t;
        let px = u*u*u*p1rx + 3.0*u*u*t*c1rx + 3.0*u*t*t*c2rx + t*t*t*p2rx;
        let py = u*u*u*p1ry + 3.0*u*u*t*c1ry + 3.0*u*t*t*c2ry + t*t*t*p2ry;
        let tdx = 3.0*(u*u*(c1rx-p1rx) + 2.0*u*t*(c2rx-c1rx) + t*t*(p2rx-c2rx));
        let tdy = 3.0*(u*u*(c1ry-p1ry) + 2.0*u*t*(c2ry-c1ry) + t*t*(p2ry-c2ry));
        let tl = (tdx*tdx + tdy*tdy).sqrt();
        let (tdx, tdy) = (tdx/tl, tdy/tl);
        ((px - tdy*hh, py + tdx*hh), (px + tdy*hh, py - tdx*hh))
    };

    let mut poly: Vec<(f64,f64)> = Vec::with_capacity(2*N + 2*C + 40);

    for i in 0..=N {
        let t = i as f64 / N as f64;
        poly.push((p0rx + t*(p1rx-p0rx) + lox*h, p0ry + t*(p1ry-p0ry) + loy*h));
    }
    poly.push((p1rx, p1ry - h));
    for i in 1..=N { let (o,_) = bez_oi(i as f64/N as f64, h); poly.push(o); }
    for i in 0..=C {
        let θ = std::f64::consts::PI * i as f64 / C as f64;
        let (c, s) = (θ.cos(), θ.sin());
        poly.push((p2rx + (c*rox + s*rtdx)*h, p2ry + (c*roy + s*rtdy)*h));
    }
    for i in (1..N).rev() { let (_,pi) = bez_oi(i as f64/N as f64, h); poly.push(pi); }

    let riv_x = p1rx + rvtdy*h; let riv_y = p1ry - rvtdx*h;
    let liv_x = p1rx + lix*h;   let liv_y = p1ry + liy*h;
    let ct_x  = (riv_x + liv_x) * 0.5;
    let ct_y  = p1ry - h * 0.6;
    for i in 0..=8 {
        let t = i as f64/8.0; let u = 1.0-t;
        poly.push((u*u*riv_x + 2.0*u*t*ct_x + t*t*liv_x,
                   u*u*riv_y + 2.0*u*t*ct_y + t*t*liv_y));
    }
    for i in (0..N).rev() {
        let t = i as f64/N as f64;
        poly.push((p0rx + t*(p1rx-p0rx) + lix*h, p0ry + t*(p1ry-p0ry) + liy*h));
    }
    for i in 0..=C {
        let θ = std::f64::consts::PI * i as f64 / C as f64;
        let (c, s) = (θ.cos(), θ.sin());
        poly.push((p0rx + (c*lix + s*lbx)*h, p0ry + (c*liy + s*lby)*h));
    }

    write!(buf, "0 0 0 rg\n").ok();
    if let Some(&(fx, fy)) = poly.first() {
        write!(buf, "{:.4} {:.4} m\n", x+fx, y_pdf_anchor+fy).ok();
        for &(px, py) in poly.iter().skip(1) {
            write!(buf, "{:.4} {:.4} l\n", x+px, y_pdf_anchor+py).ok();
        }
        write!(buf, "h f\n").ok();
    }
}

// ── Image placement ───────────────────────────────────────────────────────────

/// Encode image as XObject and emit draw ops. Returns (resource name, object id) or None.
fn write_image_ops(
    doc: &mut Document,
    buf: &mut Vec<u8>,
    img_field: &ImageField,
) -> Option<(String, lopdf::ObjectId)> {
    if img_field.data.is_empty() { return None; }

    let dyn_img = image::load_from_memory(&img_field.data).ok()?;
    let (iw, ih) = (dyn_img.width(), dyn_img.height());
    if iw == 0 || ih == 0 || (iw == 1 && ih == 1) { return None; }

    let disp_w = PAGE_WIDTH * img_field.scale;
    let disp_h = disp_w * ih as f64 / iw as f64;

    let (x, y_td) = anchor_to_xy(img_field.pos, img_field.dx, img_field.dy);
    let y_pdf = PAGE_HEIGHT - y_td - disp_h;

    let rgba = dyn_img.to_rgba8();
    let rgb: Vec<u8>   = rgba.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
    let alpha: Vec<u8> = rgba.pixels().map(|p| p[3]).collect();

    let c_rgb   = compress_zlib(&rgb);
    let c_alpha = compress_zlib(&alpha);

    // SMask
    let mut sm = Dictionary::new();
    sm.set(b"Type".to_vec(),             Object::Name(b"XObject".to_vec()));
    sm.set(b"Subtype".to_vec(),          Object::Name(b"Image".to_vec()));
    sm.set(b"Width".to_vec(),            Object::Integer(iw as i64));
    sm.set(b"Height".to_vec(),           Object::Integer(ih as i64));
    sm.set(b"ColorSpace".to_vec(),       Object::Name(b"DeviceGray".to_vec()));
    sm.set(b"BitsPerComponent".to_vec(), Object::Integer(8));
    sm.set(b"Filter".to_vec(),           Object::Name(b"FlateDecode".to_vec()));
    sm.set(b"Length".to_vec(),           Object::Integer(c_alpha.len() as i64));
    let smask_id = doc.add_object(Stream::new(sm, c_alpha));

    // Image XObject
    let xobj_name = format!("Img{}", doc.objects.len());
    let mut im = Dictionary::new();
    im.set(b"Type".to_vec(),             Object::Name(b"XObject".to_vec()));
    im.set(b"Subtype".to_vec(),          Object::Name(b"Image".to_vec()));
    im.set(b"Width".to_vec(),            Object::Integer(iw as i64));
    im.set(b"Height".to_vec(),           Object::Integer(ih as i64));
    im.set(b"ColorSpace".to_vec(),       Object::Name(b"DeviceRGB".to_vec()));
    im.set(b"BitsPerComponent".to_vec(), Object::Integer(8));
    im.set(b"Filter".to_vec(),           Object::Name(b"FlateDecode".to_vec()));
    im.set(b"Length".to_vec(),           Object::Integer(c_rgb.len() as i64));
    im.set(b"SMask".to_vec(),            Object::Reference(smask_id));
    let img_id = doc.add_object(Stream::new(im, c_rgb));

    // Emit draw operator
    write!(buf, "q\n{:.4} 0 0 {:.4} {:.4} {:.4} cm\n/{} Do\nQ\n",
           disp_w, disp_h, x, y_pdf, xobj_name).ok();

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
    fn test_anchor_to_xy_top_left() {
        let (x, y) = anchor_to_xy(Anchor::TopLeft, 10.0, 20.0);
        assert_eq!(x, 10.0);
        assert_eq!(y, -20.0);
    }

    #[test]
    fn test_anchor_to_xy_bottom_center() {
        let (x, y) = anchor_to_xy(Anchor::BottomCenter, 0.0, 100.0);
        assert!((x - PAGE_WIDTH/2.0).abs() < 0.01);
        assert!((y - (PAGE_HEIGHT - 100.0)).abs() < 0.01);
    }
}
