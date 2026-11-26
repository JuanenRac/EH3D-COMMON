//! The calls into PDFium. This is the only unsafe code of the viewer.
//!
//! The library accepted is built without V8 and without XFA (`pdfium.json`), so a document cannot
//! run a script; no form environment is made, no action is carried out and no link is followed. PDFium
//! is not safe to call from two threads at once, so every call holds one lock.
//!
//! The signatures are those of the public headers `fpdfview.h`, `fpdf_text.h` of the pinned release.
//! The 32-bit Windows library uses the C calling convention.

use std::ffi::{c_char, c_double, c_float, c_int, c_ulong, c_void};
use std::path::Path;
use std::ptr::{null, null_mut};
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::{pins, Area, Hit, PdfDocument};
use crate::error::ViewerError;
use crate::picture::Raster;

/// The longest side of a rendered page.
pub const MAX_RENDER_SIDE: u32 = 8192;
/// The most pages of a document that is opened.
const MAX_PAGES: c_int = 100_000;

type Handle = *mut c_void;

#[repr(C)]
struct LibraryConfig {
    version: c_int,
    user_font_paths: *const *const c_char,
    isolate: *mut c_void,
    v8_embedder_slot: u32,
    platform: *mut c_void,
}

macro_rules! api {
    ($($field:ident: $symbol:literal fn($($argument:ty),*) $(-> $result:ty)?;)*) => {
        #[allow(clippy::type_complexity)]
        struct Api {
            $($field: unsafe extern "C" fn($($argument),*) $(-> $result)?,)*
            // kept for as long as the functions above are used: it is never unloaded
            _library: libloading::Library,
        }

        impl Api {
            /// # Safety
            /// The library must be the PDFium whose headers declare these signatures.
            unsafe fn new(library: libloading::Library) -> Result<Api, libloading::Error> {
                Ok(Api {
                    $($field: *unsafe { library.get(concat!($symbol, "\0").as_bytes()) }?,)*
                    _library: library,
                })
            }
        }
    };
}

api! {
    init: "FPDF_InitLibraryWithConfig" fn(*const LibraryConfig);
    load_document: "FPDF_LoadMemDocument64" fn(*const c_void, usize, *const c_char) -> Handle;
    last_error: "FPDF_GetLastError" fn() -> c_ulong;
    close_document: "FPDF_CloseDocument" fn(Handle);
    page_count: "FPDF_GetPageCount" fn(Handle) -> c_int;
    load_page: "FPDF_LoadPage" fn(Handle, c_int) -> Handle;
    close_page: "FPDF_ClosePage" fn(Handle);
    page_width: "FPDF_GetPageWidthF" fn(Handle) -> c_float;
    page_height: "FPDF_GetPageHeightF" fn(Handle) -> c_float;
    page_to_device: "FPDF_PageToDevice" fn(Handle, c_int, c_int, c_int, c_int, c_int, c_double, c_double, *mut c_int, *mut c_int) -> c_int;
    bitmap_create: "FPDFBitmap_CreateEx" fn(c_int, c_int, c_int, *mut c_void, c_int) -> Handle;
    bitmap_fill: "FPDFBitmap_FillRect" fn(Handle, c_int, c_int, c_int, c_int, c_ulong) -> c_int;
    render: "FPDF_RenderPageBitmap" fn(Handle, Handle, c_int, c_int, c_int, c_int, c_int, c_int);
    bitmap_destroy: "FPDFBitmap_Destroy" fn(Handle);
    text_load: "FPDFText_LoadPage" fn(Handle) -> Handle;
    text_close: "FPDFText_ClosePage" fn(Handle);
    find_start: "FPDFText_FindStart" fn(Handle, *const u16, c_ulong, c_int) -> Handle;
    find_next: "FPDFText_FindNext" fn(Handle) -> c_int;
    find_index: "FPDFText_GetSchResultIndex" fn(Handle) -> c_int;
    find_count: "FPDFText_GetSchCount" fn(Handle) -> c_int;
    find_close: "FPDFText_FindClose" fn(Handle);
    count_rects: "FPDFText_CountRects" fn(Handle, c_int, c_int) -> c_int;
    get_rect: "FPDFText_GetRect" fn(Handle, c_int, *mut c_double, *mut c_double, *mut c_double, *mut c_double) -> c_int;
}

static CALLS: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<Result<Api, ViewerError>> = OnceLock::new();

fn lock() -> MutexGuard<'static, ()> {
    CALLS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(windows)]
fn load_library(path: &Path) -> Result<libloading::Library, libloading::Error> {
    // what the library needs is looked for beside it and in System32, nowhere else
    const SEARCH_DLL_LOAD_DIR: u32 = 0x0100;
    const SEARCH_SYSTEM32: u32 = 0x0800;
    // SAFETY: the file was checked against the pinned SHA-256 before this call
    unsafe {
        libloading::os::windows::Library::load_with_flags(
            path,
            SEARCH_DLL_LOAD_DIR | SEARCH_SYSTEM32,
        )
    }
    .map(Into::into)
}

#[cfg(not(windows))]
fn load_library(path: &Path) -> Result<libloading::Library, libloading::Error> {
    // SAFETY: the file was checked against the pinned SHA-256 before this call; a full path means
    // the search path of the system is not used for it
    unsafe { libloading::Library::new(path) }
}

fn start() -> Result<Api, ViewerError> {
    let path = pins::library_beside_program()?;
    let rejected = |e: libloading::Error| ViewerError::EngineRejected(e.to_string());
    let library = load_library(&path).map_err(rejected)?;
    // SAFETY: the pinned build is the one these signatures come from
    let api = unsafe { Api::new(library) }.map_err(rejected)?;
    let config = LibraryConfig {
        version: 2,
        user_font_paths: null(),
        isolate: null_mut(),
        v8_embedder_slot: 0,
        platform: null_mut(),
    };
    let _held = lock();
    // SAFETY: called once, before any other call, with a configuration of version 2
    unsafe { (api.init)(&config) };
    Ok(api)
}

/// The engine, started the first time it is asked for. A failure is not tried again while the
/// program runs.
fn engine() -> Result<&'static Api, ViewerError> {
    match ENGINE.get_or_init(start) {
        Ok(api) => Ok(api),
        Err(e) => Err(e.clone()),
    }
}

/// A document held by PDFium.
pub struct PdfiumDocument {
    api: &'static Api,
    document: Handle,
    pages: u32,
    // PDFium reads from these bytes while the document is open: they are never changed or moved
    _data: Vec<u8>,
}

struct Page<'a> {
    api: &'a Api,
    handle: Handle,
}

impl Drop for Page<'_> {
    fn drop(&mut self) {
        // SAFETY: the handle came from FPDF_LoadPage and is closed once; the lock is held by the
        // caller, which outlives this value
        unsafe { (self.api.close_page)(self.handle) };
    }
}

impl PdfiumDocument {
    /// Opens a document held in memory.
    pub fn open(data: Vec<u8>) -> Result<Self, ViewerError> {
        let api = engine()?;
        let _held = lock();
        // SAFETY: the pointer and the length are those of `data`, which lives as long as the document
        let document = unsafe { (api.load_document)(data.as_ptr().cast(), data.len(), null()) };
        if document.is_null() {
            // SAFETY: read just after the failed call, under the same lock
            return Err(match unsafe { (api.last_error)() } {
                4 => ViewerError::PasswordProtected,
                code => ViewerError::Damaged(format!("PDF: error {code}")),
            });
        }
        // SAFETY: the document is open
        let pages = unsafe { (api.page_count)(document) };
        if pages <= 0 || pages > MAX_PAGES {
            // SAFETY: closed once; the value is not built
            unsafe { (api.close_document)(document) };
            return Err(ViewerError::Damaged(format!("PDF: {pages} pages")));
        }
        Ok(PdfiumDocument {
            api,
            document,
            pages: pages.unsigned_abs(),
            _data: data,
        })
    }

    /// The caller holds the lock for as long as it keeps the page.
    fn page(&self, page: u32) -> Result<Page<'_>, ViewerError> {
        let bad = || ViewerError::Damaged(format!("PDF: page {}", u64::from(page) + 1));
        if page >= self.pages {
            return Err(bad());
        }
        let index = c_int::try_from(page).map_err(|_| bad())?;
        // SAFETY: the document is open and the index is within its pages
        let handle = unsafe { (self.api.load_page)(self.document, index) };
        if handle.is_null() {
            return Err(bad());
        }
        Ok(Page {
            api: self.api,
            handle,
        })
    }

    /// The areas of the characters `start..start + count`, as fractions of the page.
    ///
    /// # Safety
    /// `text` is an open text page of `page`, and the lock is held.
    unsafe fn areas(&self, page: &Page, text: Handle, start: c_int, count: c_int) -> Vec<Area> {
        const SCALE: c_int = 100_000;
        let api = self.api;
        // SAFETY: the handles are open (contract of this function); the out values are local
        let rects = unsafe { (api.count_rects)(text, start, count) }.clamp(0, 64);
        let mut areas = Vec::new();
        for index in 0..rects {
            let (mut left, mut top, mut right, mut bottom) = (0.0, 0.0, 0.0, 0.0);
            // SAFETY: as above
            let found = unsafe {
                (api.get_rect)(text, index, &mut left, &mut top, &mut right, &mut bottom)
            };
            if found == 0 {
                continue;
            }
            let mut corners = [(0, 0), (0, 0)];
            for (corner, (x, y)) in corners.iter_mut().zip([(left, top), (right, bottom)]) {
                // SAFETY: as above; this also turns the page the way it is drawn
                unsafe {
                    (api.page_to_device)(
                        page.handle,
                        0,
                        0,
                        SCALE,
                        SCALE,
                        0,
                        x,
                        y,
                        &mut corner.0,
                        &mut corner.1,
                    )
                };
            }
            let fraction = |v: c_int| (v as f32 / SCALE as f32).clamp(0.0, 1.0);
            let [a, b] = corners;
            areas.push(Area {
                left: fraction(a.0.min(b.0)),
                top: fraction(a.1.min(b.1)),
                right: fraction(a.0.max(b.0)),
                bottom: fraction(a.1.max(b.1)),
            });
        }
        areas
    }
}

impl PdfDocument for PdfiumDocument {
    fn page_count(&self) -> u32 {
        self.pages
    }

    fn page_size(&self, page: u32) -> Result<(f32, f32), ViewerError> {
        let _held = lock();
        let page = self.page(page)?;
        // SAFETY: the page is open
        Ok(unsafe {
            (
                (self.api.page_width)(page.handle),
                (self.api.page_height)(page.handle),
            )
        })
    }

    fn render(&self, page: u32, width: u32, height: u32) -> Result<Raster, ViewerError> {
        if width == 0 || height == 0 || width > MAX_RENDER_SIDE || height > MAX_RENDER_SIDE {
            return Err(ViewerError::TooLarge {
                limit: u64::from(MAX_RENDER_SIDE),
            });
        }
        let _held = lock();
        let page = self.page(page)?;
        // both sides are at most MAX_RENDER_SIDE, so none of these overflows
        let stride = width as usize * 4;
        let mut pixels = vec![0u8; stride * height as usize];
        let (w, h) = (width as c_int, height as c_int);
        // SAFETY: the buffer holds `h` rows of `stride` bytes and lives past the destruction of the
        // bitmap; format 4 is BGRA
        let bitmap = unsafe {
            (self.api.bitmap_create)(w, h, 4, pixels.as_mut_ptr().cast(), stride as c_int)
        };
        if bitmap.is_null() {
            return Err(ViewerError::Damaged("PDF: no bitmap".into()));
        }
        // SAFETY: the bitmap and the page are open; flag 1 draws the annotations
        unsafe {
            (self.api.bitmap_fill)(bitmap, 0, 0, w, h, 0xFFFF_FFFF);
            (self.api.render)(bitmap, page.handle, 0, 0, w, h, 0, 1);
            (self.api.bitmap_destroy)(bitmap);
        }
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        Ok(Raster {
            width,
            height,
            rgba: pixels,
        })
    }

    fn find(&self, page: u32, query: &str, max: usize) -> Result<Vec<Hit>, ViewerError> {
        let mut hits = Vec::new();
        if query.is_empty() || query.contains('\0') || max == 0 {
            return Ok(hits);
        }
        let wide: Vec<u16> = query.encode_utf16().chain(std::iter::once(0)).collect();
        let _held = lock();
        let number = page;
        let page = self.page(page)?;
        // SAFETY: the page is open
        let text = unsafe { (self.api.text_load)(page.handle) };
        if text.is_null() {
            return Ok(hits);
        }
        // SAFETY: `wide` ends with a zero and lives past the search
        let search = unsafe { (self.api.find_start)(text, wide.as_ptr(), 0, 0) };
        if !search.is_null() {
            // SAFETY: the search, the text page and the page are open and the lock is held
            unsafe {
                while hits.len() < max && (self.api.find_next)(search) != 0 {
                    let start = (self.api.find_index)(search);
                    let count = (self.api.find_count)(search);
                    hits.push(Hit {
                        page: number,
                        areas: self.areas(&page, text, start, count),
                    });
                }
                (self.api.find_close)(search);
            }
        }
        // SAFETY: closed once, before its page
        unsafe { (self.api.text_close)(text) };
        Ok(hits)
    }
}

impl Drop for PdfiumDocument {
    fn drop(&mut self) {
        let _held = lock();
        // SAFETY: the document is open and closed once; its bytes are freed after this
        unsafe { (self.api.close_document)(self.document) };
    }
}
