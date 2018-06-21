use std::cmp;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::io::Read;
use std::path;

use gfx_glyph::FontId;
use image;
use image::RgbaImage;

use super::*;

/// A font that defines the shape of characters drawn on the screen.
/// Can be created from a .ttf file or from an image (bitmap fonts).
#[derive(Clone)]
pub enum Font {
    /// A bitmap font where letter widths are infered
    BitmapFontVariant(BitmapFont),
    /// A TrueType font stored in `GraphicsContext::glyph_brush`
    GlyphFont(FontId),
}

/// A bitmap font where letter widths are infered
#[derive(Clone, Debug)]
pub struct BitmapFont {
    /// The original glyph image
    bytes: Vec<u8>,
    /// Width of the image
    width: usize,
    /// Height of the image (same as the height of a glyph)
    height: usize,
    /// Glyph to horizontal position (in pixels) and span (in pixels) (does not include space)
    glyphs: BTreeMap<char, (usize, usize)>,
    /// Width in pixels of the space
    space_width: usize,
    letter_separation: usize,
}

/// A mapping from character to glyph location for bitmap fonts.
/// All coordinates are stored in the span `[0-1]`.
#[derive(Debug, Clone)]
pub struct BitmapFontLayout {
    /// The layout information for each character.
    pub mapping: HashMap<char, Rect>,
}

impl BitmapFontLayout {
    /// Creates a new `BitmapFontLayout` by assuming that
    /// the characters form a uniform grid with the glyphs
    /// given by the string going from left to right, top to bottom,
    /// and the size of a grid cell is given by `rect` (in the span
    /// `[0-1]`)
    /// 
    /// Because the grid cells are in `[0-1]` this doesn't need to know
    /// how big the actual image is.
    /// 
    /// TODO: This coordinate system is inconsistent with SpriteBatch.  :-/
    fn uniform(s: &str, rect: Rect) {
        // TODO
    }

    /// Takes a something implementing `IntoIterator` and creates a
    /// `BitmapFontLayout` with the items it yields.
    /// 
    /// TODO: Implement FromIterator?
    fn from_specification<T>(iter: T)
    where T: IntoIterator<Item=(char, Rect)> {
        // TODO
    }
}

impl BitmapFont {
    fn span_for(&self, c: char) -> usize {
        match self.glyphs.get(&c) {
            Some(&(_, span)) => span,
            None => {
                if c == ' ' {
                    self.space_width
                } else {
                    0
                    //No span is defined for this char.
                    // We could error here, but I don't see the point.
                    // We will just render the missing char as nothing and move on,
                    // and the user will see that there is a nothing and if they
                    // do not understand, they will certainly feel silly when
                    // we and ask them what they expected to happen when they
                    // told the system to render a char they never specified. I t
                    // hink I would kind of prefer an implementation that is
                    // guaranteed not to error for any string.
                    // TODO: While this is a perfectly valid preference, I would
                    // prefer fail-noisily to fail-invisibly; we should possibly have
                    // options for either behavior.
                }
            }
        }
    }
}

impl Font {
    /// Load a new TTF font from the given file.
    pub fn new<P>(context: &mut Context, path: P, points: u32) -> GameResult<Font>
    where
        P: AsRef<path::Path> + fmt::Debug,
    {
        let name = format!("{:?}", path);

        // TODO: consider ditching DPI here; wait for winit #548.
        Font::new_glyph_font(context, path)
    }

/*
    /// Load a new TTF font from the given file, returning a font that draws
    /// lines that are the given number of pixels high.
    /// TODO: figure out how to make this better with GlyphBrush
    pub fn new_px<P>(context: &mut Context, path: P, pixels: u32) -> GameResult<Font>
    where
        P: AsRef<path::Path> + fmt::Debug,
    {
        let mut stream = context.filesystem.open(path.as_ref())?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;

        let name = format!("{:?}", path);

        Font::from_bytes_px(&name, &buf, pixels)
    }
*/

    /// Creates a bitmap font from a long image of its alphabet, specified by `path`.
    /// The width of each individual chars is assumed to be to be
    /// image(path).width/glyphs.chars().count()
    pub fn new_bitmap<P: AsRef<path::Path>>(
        context: &mut Context,
        path: P,
        glyphs: &str,
    ) -> GameResult<Font> {
        let img = {
            let mut buf = Vec::new();
            let mut reader = context.filesystem.open(path)?;
            reader.read_to_end(&mut buf)?;
            image::load_from_memory(&buf)?.to_rgba()
        };
        let (image_width, image_height) = img.dimensions();

        let glyph_width = (image_width as usize) / glyphs.len();
        let mut glyphs_map: BTreeMap<char, (usize, usize)> = BTreeMap::new();
        for (i, c) in glyphs.chars().enumerate() {
            glyphs_map.insert(c, (i * glyph_width, glyph_width));
        }
        Ok(Font::BitmapFontVariant(BitmapFont {
            bytes: img.into_vec(),
            width: image_width as usize,
            height: image_height as usize,
            glyphs: glyphs_map,
            space_width: glyph_width,
            letter_separation: 0,
        }))
    }

    /// Creates a bitmap font from a long image of its alphabet.
    /// Each letter must be separated from the last by a fully transparent column of pixels.
    /// The width of each letter is infered from these letter boundaries.
    pub fn new_variable_width_bitmap_font<P: AsRef<path::Path>>(
        context: &mut Context,
        path: P,
        glyphs: &str,
        space_width: usize, //in addition to letter_separation
        letter_separation: usize,
    ) -> GameResult<Font> {
        let img = {
            let mut buf = Vec::new();
            let mut reader = context.filesystem.open(path)?;
            reader.read_to_end(&mut buf)?;
            image::load_from_memory(&buf)?.to_rgba()
        };
        let (image_width, image_height) = img.dimensions();

        let mut glyphs_map: BTreeMap<char, (usize, usize)> = BTreeMap::new();
        let mut start = 0usize;
        let mut glyphos = glyphs.chars().enumerate();
        let column_has_content = |offset: usize, image: &RgbaImage| {
            //iff any pixel herein has an alpha greater than 0
            (0..image_height).any(|ir| image.get_pixel(offset as u32, ir).data[3] > 0)
        };
        while start < image_width as usize {
            if column_has_content(start, &img) {
                let mut span = 1;
                while start + span < image_width as usize && column_has_content(start + span, &img)
                {
                    span += 1;
                }
                let next_char: char = glyphos
                    .next()
                    .ok_or_else(|| {
                        GameError::FontError("I counted more glyphs in the font bitmap than there were chars in the glyphs string. Note, glyphs must not have gaps. A glyph with a transparent column in the middle will read as two glyphs.".into())
                    })?
                    .1;
                glyphs_map.insert(next_char, (start, span));
                start += span;
            }
            start += 1;
        }

        let (lb, _) = glyphos.size_hint();
        if lb > 0 {
            return Err(GameError::FontError(
                "There were more chars in glyphs than I counted in the bitmap!".into(),
            ));
        }

        Ok(Font::BitmapFontVariant(BitmapFont {
            bytes: img.into_vec(),
            width: image_width as usize,
            height: image_height as usize,
            glyphs: glyphs_map,
            space_width,
            letter_separation,
        }))
    }

    /// Loads a new TrueType font from given bytes and into `GraphicsContext::glyph_brush`.
    pub fn new_glyph_font_bytes(context: &mut Context, bytes: &[u8]) -> GameResult<Self>
    {
        // TODO: Take a Cow here to avoid this clone where unnecessary?
        let v = bytes.to_vec();
        let font_id = context.gfx_context.glyph_brush.add_font_bytes(v);

        Ok(Font::GlyphFont(font_id))
    }

    /// Loads a new TrueType font from given file and into `GraphicsContext::glyph_brush`.
    pub fn new_glyph_font<P>(context: &mut Context, path: P) -> GameResult<Self>
    where
        P: AsRef<path::Path> + fmt::Debug,
    {
        let mut stream = context.filesystem.open(path.as_ref())?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;

        Font::new_glyph_font_bytes(context, &buf)
    }

    /// Retrieves a loaded font from `GraphicsContext::glyph_brush`.
    pub fn get_glyph_font_by_id(context: &mut Context, font_id: FontId) -> GameResult<Self> {
        if context
            .gfx_context
            .glyph_brush
            .fonts()
            .contains_key(&font_id)
        {
            Ok(Font::GlyphFont(font_id))
        } else {
            Err(GameError::FontError(
                format!("Font {:?} not found!", font_id).into(),
            ))
        }
    }

    /// Returns the baked-in bytes of default font (currently DejaVuSerif.ttf).
    pub(crate) fn default_font_bytes() -> &'static [u8] {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/DejaVuSerif.ttf"
        ))
    }

    /// Returns baked-in default font (currently DejaVuSerif.ttf).
    /// Note it does create a new `Font` object with every call;
    /// although the actual data should be shared.
    pub fn default_font(context: &mut Context) -> GameResult<Self> {
        // BUGGO: fix DPI.  Get from Context?  If we do that we can basically
        // just make Context always keep the default Font itself... hmm.
        // TODO: ^^^ is that still relevant?  Nah, it will probably be replaced by
        // the `gfx_glyph` interation.
        Font::new_glyph_font_bytes(context, Font::default_font_bytes())
    }

    /// Get the height of the Font in pixels.
    ///
    /// The height of the font includes any spacing, it will be the total height
    /// a line needs.
    /// TODO: Probably made obsolete by GlyphFont
    pub fn get_height(&self) -> usize {
        match *self {
            Font::BitmapFontVariant(BitmapFont { height, .. }) => height,
            Font::GlyphFont(_) => 0,
        }
    }

    /// Returns the width a line of text needs, in pixels.
    /// Does not handle line-breaks.
    /// TODO: Probably made obsolete by GlyphFont
    pub fn get_width(&self, text: &str) -> usize {
        match *self {
            Font::BitmapFontVariant(ref font) => {
                compute_variable_bitmap_text_rendering_span(text, font)
            }
            Font::GlyphFont(_) => 0,
        }
    }

    /// Breaks the given text into lines that will not exceed `wrap_limit` pixels
    /// in length when drawn with the given font.
    /// It accounts for newlines correctly but does not
    /// try to break words or handle hyphenated words; it just breaks
    /// at whitespace.  (It also doesn't preserve whitespace.)
    ///
    /// Returns a tuple of maximum line width and a `Vec` of wrapped `String`s.
    /// TODO: Probably made obsolete by GlyphFont
    pub fn get_wrap(&self, text: &str, wrap_limit: usize) -> (usize, Vec<String>) {
        let mut broken_lines = Vec::new();
        for line in text.lines() {
            let mut current_line = Vec::new();
            for word in line.split_whitespace() {
                // I'm sick of trying to do things the clever way and
                // build up a line word by word while tracking how
                // long it should be, so instead I just re-render the whole
                // line, incrementally adding a word at a time until it
                // becomes too long.
                // This is not the most efficient way but it is simple and
                // it works.
                let mut prospective_line = current_line.clone();
                prospective_line.push(word);
                let text = prospective_line.join(" ");
                let prospective_line_width = self.get_width(&text);
                if prospective_line_width > wrap_limit {
                    // Current line is long enough, keep it
                    broken_lines.push(current_line.join(" "));
                    // and overflow the current word onto the next line.
                    current_line.clear();
                    current_line.push(word);
                } else {
                    // Current line with the added word is still short enough
                    current_line.push(word);
                }
            }

            // Push the last line of the text
            broken_lines.push(current_line.join(" "));
        }

        // If we have a line with only whitespace on it,
        // this results in the unwrap_or value.
        // And we can't create a texture of size 0, so
        // we put 1 here.
        // Not entirely sure what this will actually result
        // in though; hopefully a blank line.
        let max_line_length = broken_lines
            .iter()
            .map(|line| self.get_width(line))
            .max()
            .unwrap_or(1);

        (max_line_length, broken_lines)
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Font::BitmapFontVariant(BitmapFont { .. }) => write!(f, "<BitmapFont: {:p}>", &self),
            Font::GlyphFont { .. } => write!(f, "<GlyphFont: {:p}>", &self),
        }
    }
}

/// Drawable text created from a `Font`.
#[derive(Clone)]
pub struct Text {
    texture: Image,
    contents: String,
    blend_mode: Option<BlendMode>,
}

/// Treats src and dst as row-major 2D arrays, and blits the given rect from src to dst.
/// Does no bounds checking or anything; if you feed it invalid bounds it will just panic.
/// Generally, you shouldn't need to use this directly.
#[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
fn blit(
    dst: &mut [u8],
    dst_dims: (usize, usize),
    dst_point: (usize, usize),
    src: &[u8],
    src_dims: (usize, usize),
    src_point: (usize, usize),
    rect_size: (usize, usize),
    pitch: usize,
) {
    // The rect properties are all f32's; we truncate them down to integers.
    let area_row_width = rect_size.0 * pitch;
    let src_row_width = src_dims.0 * pitch;
    let dst_row_width = dst_dims.0 * pitch;

    for row_idx in 0..rect_size.1 {
        let src_row = row_idx + src_point.1;
        let dst_row = row_idx + dst_point.1;
        let src_offset = src_row * src_row_width + (src_point.0 * pitch);
        let dst_offset = dst_row * dst_row_width + (dst_point.0 * pitch);

        // println!("from {} to {}, width {}",
        //          dst_offset,
        //          src_offset,
        //          area_row_width);
        let dst_slice = &mut dst[dst_offset..(dst_offset + area_row_width)];
        let src_slice = &src[src_offset..(src_offset + area_row_width)];
        dst_slice.copy_from_slice(src_slice);
    }
}

struct VariableFontCharIter<'a> {
    font: &'a BitmapFont,
    iter: ::std::str::Chars<'a>,
    offset: usize,
}

impl<'a> Iterator for VariableFontCharIter<'a> {
    // iterates over each char in a line of text, finding the horizontal
    // offsets at which they will appear on the screen, relative to the origin.
    type Item = (char, usize, usize); //(letter, offset, letter_render_span)
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(c) = self.iter.next() {
            let char_span = self.font.span_for(c);
            let this_offset = self.offset;
            self.offset += char_span + self.font.letter_separation;
            Some((c, this_offset, char_span))
        } else {
            None
        }
    }
}

impl<'a> VariableFontCharIter<'a> {
    fn new(text: &'a str, font: &'a BitmapFont) -> VariableFontCharIter<'a> {
        VariableFontCharIter {
            font,
            iter: text.chars(),
            offset: 0,
        }
    }
}

fn compute_variable_bitmap_text_rendering_span(text: &str, font: &BitmapFont) -> usize {
    VariableFontCharIter::new(text, font)
        .last()
        .map(|(_, offset, span)| offset + span)
        .unwrap_or(0)
}

fn render_dynamic_bitmap(context: &mut Context, text: &str, font: &BitmapFont) -> GameResult<Text> {
    let image_span = compute_variable_bitmap_text_rendering_span(text, font);
    // Same at-least-one-pixel-wide constraint here as with TTF fonts.
    let buf_len = cmp::max(image_span * font.height * 4, 1);
    let mut dest_buf = Vec::with_capacity(buf_len);
    dest_buf.resize(buf_len, 0u8);
    for (c, offset, _) in VariableFontCharIter::new(text, font) {
        let (coffset, cspan) = *font.glyphs.get(&c).unwrap_or(&(0, 0));
        blit(
            &mut dest_buf,
            (image_span, font.height),
            (offset, 0),
            &font.bytes,
            (font.width, font.height),
            (coffset, 0),
            (cspan, font.height),
            4,
        );
    }

    let image = Image::from_rgba8(context, image_span as u16, font.height as u16, &dest_buf)?;
    let text_string = text.to_string();

    Ok(Text {
        texture: image,
        contents: text_string,
        blend_mode: None,
    })
}

impl Text {
    /// Renders a new `Text` from the given `Font`.
    ///
    /// Note that this is relatively computationally expensive;
    /// if you want to draw text every frame you probably want to save
    /// it and only update it when the text changes.
    pub fn new(context: &mut Context, text: &str, font: &Font) -> GameResult<Text> {
        match *font {
            Font::BitmapFontVariant(ref font) => render_dynamic_bitmap(context, text, font),
            Font::GlyphFont(_) => Err(GameError::FontError(
                "`Text` can't be created with a `Font::GlyphFont` (yet)!".into(),
            )),
        }
    }

    /// Returns the width of the rendered text, in pixels.
    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    /// Returns the height of the rendered text, in pixels.
    pub fn height(&self) -> u32 {
        self.texture.height()
    }

    /// Returns the string that the text represents.
    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// Returns the dimensions of the rendered text.
    pub fn get_dimensions(&self) -> Rect {
        self.texture.get_dimensions()
    }

    /// Get the filter mode for the the rendered text.
    pub fn get_filter(&self) -> FilterMode {
        self.texture.get_filter()
    }

    /// Set the filter mode for the the rendered text.
    pub fn set_filter(&mut self, mode: FilterMode) {
        self.texture.set_filter(mode);
    }

    /// Returns a reference to the `Image` contained
    /// by the `Text` object.
    pub fn get_image(&self) -> &Image {
        &self.texture
    }

    /// Returns a mutable  reference to the `Image` contained
    /// by the `Text` object.
    pub fn get_image_mut(&mut self) -> &mut Image {
        &mut self.texture
    }

    /// Unwraps the `Image` contained
    /// by the `Text` object.
    pub fn into_inner(self) -> Image {
        self.texture
    }
}

impl Drawable for Text {
    fn draw_primitive(&self, ctx: &mut Context, param: PrimitiveDrawParam) -> GameResult {
        draw_primitive(ctx, &self.texture, param)
    }
    fn set_blend_mode(&mut self, mode: Option<BlendMode>) {
        self.blend_mode = mode;
    }
    fn get_blend_mode(&self) -> Option<BlendMode> {
        self.blend_mode
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<Text: {}x{}, {:p}>",
            self.texture.width, self.texture.height, &self
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_blit() {
        let dst = &mut [0; 125][..];
        let src = &[
            1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 9, 1, 9, 9, 9, 9, 9,
            9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ][..];
        assert_eq!(src.len(), 25 * 5);

        // Test just blitting the whole thing
        let rect_dims = (25, 5);
        blit(dst, rect_dims, (0, 0), src, rect_dims, (0, 0), (25, 5), 1);
        //println!("{:?}", src);
        //println!("{:?}", dst);
        assert_eq!(dst, src);
        for i in 0..dst.len() {
            dst[i] = 0;
        }

        // Test blitting the whole thing with a non-1 pitch
        let rect_dims = (5, 5);
        blit(dst, rect_dims, (0, 0), src, rect_dims, (0, 0), (5, 5), 5);
        assert_eq!(dst, src);
    }
/*
    #[test]
    fn test_metrics() {
        let f = Font::default_font().expect("Could not get default font");
        assert_eq!(f.get_height(), 17);
        assert_eq!(f.get_width("Foo!"), 33);

        // http://www.catipsum.com/index.php
        let text_to_wrap = "Walk on car leaving trail of paw prints on hood and windshield sniff \
                            other cat's butt and hang jaw half open thereafter for give attitude. \
                            Annoy kitten\nbrother with poking. Mrow toy mouse squeak roll over. \
                            Human give me attention meow.";
        let (len, v) = f.get_wrap(text_to_wrap, 250);
        println!("{} {:?}", len, v);
        assert_eq!(len, 249);

        /*
        let wrapped_text = vec![
            "Walk on car leaving trail of paw prints",
            "on hood and windshield sniff other",
            "cat\'s butt and hang jaw half open",
            "thereafter for give attitude. Annoy",
            "kitten",
            "brother with poking. Mrow toy",
            "mouse squeak roll over. Human give",
            "me attention meow."
        ];
*/
        let wrapped_text = vec![
            "Walk on car leaving trail of paw",
            "prints on hood and windshield",
            "sniff other cat\'s butt and hang jaw",
            "half open thereafter for give",
            "attitude. Annoy kitten",
            "brother with poking. Mrow toy",
            "mouse squeak roll over. Human",
            "give me attention meow.",
        ];

        assert_eq!(&v, &wrapped_text);
    }

    // We sadly can't have this test in the general case because it needs to create a Context,
    // which creates a window, which fails on a headless server like our CI systems.  :/
    //#[test]
    #[allow(dead_code)]
    fn test_wrapping() {
        use conf;
        let c = conf::Conf::new();
        let (ctx, _) = &mut Context::load_from_conf("test_wrapping", "ggez", c)
            .expect("Could not create context?");
        let font = Font::default_font().expect("Could not get default font");
        let text_to_wrap = "Walk on car leaving trail of paw prints on hood and windshield sniff \
                            other cat's butt and hang jaw half open thereafter for give attitude. \
                            Annoy kitten\nbrother with poking. Mrow toy mouse squeak roll over. \
                            Human give me attention meow.";
        let wrap_length = 250;
        let (len, v) = font.get_wrap(text_to_wrap, wrap_length);
        assert!(len < wrap_length);
        for line in &v {
            let t = Text::new(ctx, line, &font).unwrap();
            println!(
                "Width is claimed to be <= {}, should be <= {}, is {}",
                len,
                wrap_length,
                t.width()
            );
            // Why does this not match?  x_X
            //assert!(t.width() as usize <= len);
            assert!(t.width() as usize <= wrap_length);
        }
    }
    */
}
