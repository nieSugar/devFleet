use crate::models::{CustomEditor, EditorCache, EditorLaunch, EditorSource, EditorView};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use std::collections::{HashMap, VecDeque};
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const ICON_SIZE: u32 = 64;
const MAX_DATA_URL_BYTES: usize = 128 * 1024;
const MAX_CACHE_ENTRIES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    source: PathBuf,
    modified: SystemTime,
    source_len: u64,
    output_size: u32,
}

impl CacheKey {
    fn from_metadata(source: &Path, metadata: &Metadata, output_size: u32) -> Option<Self> {
        Some(Self {
            source: source.to_path_buf(),
            modified: metadata.modified().ok()?,
            source_len: metadata.len(),
            output_size,
        })
    }
}

#[derive(Default)]
struct IconCache {
    values: HashMap<CacheKey, Option<String>>,
    order: VecDeque<CacheKey>,
}

impl IconCache {
    fn get(&self, key: &CacheKey) -> Option<Option<String>> {
        self.values.get(key).cloned()
    }

    fn insert(&mut self, key: CacheKey, value: Option<String>) {
        if let Some(cached) = self.values.get_mut(&key) {
            *cached = value;
            return;
        }

        while self.values.len() >= MAX_CACHE_ENTRIES {
            if let Some(oldest) = self.order.pop_front() {
                self.values.remove(&oldest);
            } else {
                self.values.clear();
                break;
            }
        }
        self.order.push_back(key.clone());
        self.values.insert(key, value);
    }

    fn clear(&mut self) {
        self.values.clear();
        self.order.clear();
    }
}

fn cache() -> &'static Mutex<IconCache> {
    static CACHE: OnceLock<Mutex<IconCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(IconCache::default()))
}

fn encode_png_data_url(rgba: &[u8], size: u32) -> Option<String> {
    let expected_len = usize::try_from(size.checked_mul(size)?.checked_mul(4)?).ok()?;
    if rgba.len() != expected_len {
        return None;
    }

    let mut encoded_png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded_png, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
    }

    let data_url = format!(
        "data:image/png;base64,{}",
        BASE64_STANDARD.encode(encoded_png)
    );
    (data_url.len() <= MAX_DATA_URL_BYTES).then_some(data_url)
}

fn load_with<F>(
    cache: &Mutex<IconCache>,
    source: &Path,
    output_size: u32,
    extract: F,
) -> Option<String>
where
    F: FnOnce(&Path, u32) -> Option<Vec<u8>>,
{
    let key = std::fs::metadata(source)
        .ok()
        .and_then(|metadata| CacheKey::from_metadata(source, &metadata, output_size));

    if let Some(key) = key.as_ref() {
        if let Some(cached) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(key) {
            return cached;
        }
    }

    let value =
        extract(source, output_size).and_then(|rgba| encode_png_data_url(&rgba, output_size));
    if let Some(key) = key {
        cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, value.clone());
    }
    value
}

fn icon_data_url(source: &Path) -> Option<String> {
    load_with(cache(), source, ICON_SIZE, extract_icon_rgba)
}

pub fn clear_cache() {
    cache().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

fn project_auto_editors_with<F>(
    cache: &EditorCache,
    mut load_icon: F,
) -> HashMap<String, EditorView>
where
    F: FnMut(&str) -> Option<String>,
{
    cache
        .iter()
        .map(|(id, editor)| {
            let (path, args) = match editor.launch.as_ref() {
                Some(EditorLaunch::Executable { path, args, .. }) => {
                    (Some(path.clone()), args.clone())
                }
                Some(EditorLaunch::MacApp { path })
                | Some(EditorLaunch::DesktopEntry { path })
                | Some(EditorLaunch::KnownWindowsBatch { path, .. }) => {
                    (Some(path.clone()), Vec::new())
                }
                None => (None, Vec::new()),
            };
            let icon = editor
                .installed
                .then(|| editor.icon_source.as_deref().and_then(&mut load_icon))
                .flatten();
            (
                id.clone(),
                EditorView {
                    name: editor.name.clone(),
                    installed: editor.installed,
                    source: EditorSource::Auto,
                    icon,
                    path,
                    args,
                    can_edit_args: false,
                },
            )
        })
        .collect()
}

pub fn project_auto_editors(cache: &EditorCache) -> HashMap<String, EditorView> {
    project_auto_editors_with(cache, |source| icon_data_url(Path::new(source)))
}

pub fn project_custom_editor(editor: &CustomEditor, installed: bool) -> EditorView {
    let (path, args, can_edit_args) = match &editor.launch {
        EditorLaunch::Executable { path, args, .. } => (path.clone(), args.clone(), true),
        EditorLaunch::MacApp { path }
        | EditorLaunch::DesktopEntry { path }
        | EditorLaunch::KnownWindowsBatch { path, .. } => (path.clone(), Vec::new(), false),
    };
    let icon = installed
        .then(|| {
            editor
                .icon_source
                .as_deref()
                .and_then(|source| icon_data_url(Path::new(source)))
        })
        .flatten();
    EditorView {
        name: editor.name.clone(),
        installed,
        source: EditorSource::Custom,
        icon,
        path: Some(path),
        args,
        can_edit_args,
    }
}

#[cfg(target_os = "windows")]
fn windows_icon_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(target_os = "windows")]
fn extract_icon_rgba(source: &Path, output_size: u32) -> Option<Vec<u8>> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};

    let _guard = windows_icon_lock()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let byte_len = usize::try_from(output_size.checked_mul(output_size)?.checked_mul(4)?).ok()?;
    if output_size == 0 || output_size > i32::MAX as u32 || byte_len > u32::MAX as usize {
        return None;
    }
    let wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut file_info = SHFILEINFOW::default();
    let found = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut file_info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if found == 0 || file_info.hIcon.is_invalid() {
        return None;
    }

    let icon = file_info.hIcon;
    let rendered = unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            None
        } else {
            let bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: output_size as i32,
                    biHeight: -(output_size as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: byte_len as u32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            match CreateDIBSection(Some(dc), &bitmap_info, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(bitmap) => {
                    if bits.is_null() {
                        let _ = DeleteObject(HGDIOBJ::from(bitmap));
                        let _ = DeleteDC(dc);
                        None
                    } else {
                        let old = SelectObject(dc, HGDIOBJ::from(bitmap));
                        let result = if old.is_invalid() {
                            None
                        } else {
                            std::ptr::write_bytes(bits, 0, byte_len);
                            let drawn = DrawIconEx(
                                dc,
                                0,
                                0,
                                icon,
                                output_size as i32,
                                output_size as i32,
                                0,
                                None,
                                DI_NORMAL,
                            );
                            if drawn.is_ok() && GdiFlush().as_bool() {
                                let mut rgba =
                                    std::slice::from_raw_parts(bits.cast::<u8>(), byte_len)
                                        .to_vec();
                                for pixel in rgba.chunks_exact_mut(4) {
                                    pixel.swap(0, 2);
                                }
                                Some(rgba)
                            } else {
                                None
                            }
                        };
                        if !old.is_invalid() {
                            let _ = SelectObject(dc, old);
                        }
                        let _ = DeleteObject(HGDIOBJ::from(bitmap));
                        let _ = DeleteDC(dc);
                        result
                    }
                }
                _ => {
                    let _ = DeleteDC(dc);
                    None
                }
            }
        }
    };
    unsafe {
        let _ = DestroyIcon(icon);
    }
    rendered
}

#[cfg(target_os = "macos")]
fn extract_icon_rgba(source: &Path, output_size: u32) -> Option<Vec<u8>> {
    use objc2::AnyThread;
    use objc2_app_kit::{NSBitmapImageRep, NSDeviceRGBColorSpace, NSGraphicsContext, NSWorkspace};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    let size = usize::try_from(output_size).ok()?;
    let byte_len = size.checked_mul(size)?.checked_mul(4)?;
    let path = NSString::from_str(source.to_str()?);
    let image = NSWorkspace::sharedWorkspace().iconForFile(&path);
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            size as isize,
            size as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )?
    };
    let bitmap_data = bitmap.bitmapData();
    if bitmap_data.is_null() {
        return None;
    }
    unsafe {
        std::ptr::write_bytes(bitmap_data, 0, byte_len);
    }

    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)?;
    let previous = NSGraphicsContext::currentContext();
    NSGraphicsContext::setCurrentContext(Some(&context));

    let image_size = image.size();
    let (width, height) = if image_size.width > 0.0 && image_size.height > 0.0 {
        let scale =
            (output_size as f64 / image_size.width).min(output_size as f64 / image_size.height);
        (image_size.width * scale, image_size.height * scale)
    } else {
        (output_size as f64, output_size as f64)
    };
    let rect = NSRect::new(
        NSPoint::new(
            (output_size as f64 - width) / 2.0,
            (output_size as f64 - height) / 2.0,
        ),
        NSSize::new(width, height),
    );
    image.drawInRect(rect);
    NSGraphicsContext::setCurrentContext(previous.as_deref());

    Some(unsafe { std::slice::from_raw_parts(bitmap_data, byte_len) }.to_vec())
}

#[cfg(target_os = "linux")]
fn extract_icon_rgba(source: &Path, output_size: u32) -> Option<Vec<u8>> {
    use gio::prelude::*;
    use gtk::prelude::*;

    let icon_path = if source
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
    {
        let app = gio::DesktopAppInfo::from_filename(source)?;
        let icon = app.icon()?;
        if let Ok(file_icon) = icon.clone().downcast::<gio::FileIcon>() {
            file_icon.file().path()?
        } else {
            gtk::IconTheme::default()?
                .lookup_by_gicon(
                    &icon,
                    i32::try_from(output_size).ok()?,
                    gtk::IconLookupFlags::empty(),
                )?
                .filename()?
        }
    } else {
        source.to_path_buf()
    };
    if !icon_path.is_absolute() || !icon_path.is_file() {
        return None;
    }

    let size = i32::try_from(output_size).ok()?;
    let pixbuf = gdk_pixbuf::Pixbuf::from_file_at_scale(&icon_path, size, size, true).ok()?;
    let width = usize::try_from(pixbuf.width()).ok()?;
    let height = usize::try_from(pixbuf.height()).ok()?;
    let channels = usize::try_from(pixbuf.n_channels()).ok()?;
    let rowstride = usize::try_from(pixbuf.rowstride()).ok()?;
    let output_size = usize::try_from(output_size).ok()?;
    if width == 0
        || height == 0
        || width > output_size
        || height > output_size
        || !matches!(channels, 3 | 4)
    {
        return None;
    }

    let bytes = pixbuf.read_pixel_bytes();
    let pixels = bytes.as_ref();
    let required = rowstride
        .checked_mul(height.saturating_sub(1))?
        .checked_add(width.checked_mul(channels)?)?;
    if pixels.len() < required {
        return None;
    }

    let mut rgba = vec![0; output_size.checked_mul(output_size)?.checked_mul(4)?];
    let x_offset = (output_size - width) / 2;
    let y_offset = (output_size - height) / 2;
    for y in 0..height {
        for x in 0..width {
            let source_offset = y * rowstride + x * channels;
            let output_offset = ((y + y_offset) * output_size + x + x_offset) * 4;
            rgba[output_offset..output_offset + 3]
                .copy_from_slice(&pixels[source_offset..source_offset + 3]);
            rgba[output_offset + 3] = if channels == 4 {
                pixels[source_offset + 3]
            } else {
                255
            };
        }
    }
    Some(rgba)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn extract_icon_rgba(_source: &Path, _output_size: u32) -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EditorInfo;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn rgba(size: u32) -> Vec<u8> {
        let mut pixels = vec![255; (size * size * 4) as usize];
        pixels[..4].copy_from_slice(&[10, 20, 30, 0]);
        pixels
    }

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "devfleet-icons-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(name);
        std::fs::write(&file, b"icon source").unwrap();
        file
    }

    fn decode(data_url: &str) -> (png::OutputInfo, Vec<u8>) {
        let encoded = data_url.strip_prefix("data:image/png;base64,").unwrap();
        let bytes = BASE64_STANDARD.decode(encoded).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut pixels).unwrap();
        let used = info.buffer_size();
        pixels.truncate(used);
        (info, pixels)
    }

    #[test]
    fn png_data_url_has_expected_size_and_alpha() {
        let data_url = encode_png_data_url(&rgba(ICON_SIZE), ICON_SIZE).unwrap();
        assert!(data_url.len() <= MAX_DATA_URL_BYTES);
        let (info, pixels) = decode(&data_url);
        assert_eq!((info.width, info.height), (ICON_SIZE, ICON_SIZE));
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(pixels[..4], [10, 20, 30, 0]);
    }

    #[test]
    fn success_and_failure_are_cached_until_clear() {
        let source = temp_file("cache.exe");
        let cache = Mutex::new(IconCache::default());
        let calls = AtomicUsize::new(0);
        let first = load_with(&cache, &source, ICON_SIZE, |_, size| {
            calls.fetch_add(1, Ordering::SeqCst);
            Some(rgba(size))
        });
        let second = load_with(&cache, &source, ICON_SIZE, |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            None
        });
        assert_eq!(first, second);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        cache.lock().unwrap().clear();
        assert!(load_with(&cache, &source, ICON_SIZE, |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            None
        })
        .is_none());
        assert!(load_with(&cache, &source, ICON_SIZE, |_, size| {
            calls.fetch_add(1, Ordering::SeqCst);
            Some(rgba(size))
        })
        .is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        cache.lock().unwrap().clear();
        assert!(load_with(&cache, &source, ICON_SIZE, |_, size| {
            calls.fetch_add(1, Ordering::SeqCst);
            Some(rgba(size))
        })
        .is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        std::fs::remove_dir_all(source.parent().unwrap()).unwrap();
    }

    #[test]
    fn metadata_change_invalidates_cached_value() {
        let source = temp_file("mtime.exe");
        let cache = Mutex::new(IconCache::default());
        let calls = AtomicUsize::new(0);
        let load = |_: &Path, size| {
            calls.fetch_add(1, Ordering::SeqCst);
            Some(rgba(size))
        };
        assert!(load_with(&cache, &source, ICON_SIZE, load).is_some());

        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&source)
            .unwrap();
        let changed = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        file.set_times(std::fs::FileTimes::new().set_modified(changed))
            .unwrap();
        assert!(load_with(&cache, &source, ICON_SIZE, load).is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        std::fs::remove_dir_all(source.parent().unwrap()).unwrap();
    }

    #[test]
    fn unavailable_metadata_is_never_cached() {
        let source = std::env::temp_dir().join(format!(
            "devfleet-missing-icon-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let cache = Mutex::new(IconCache::default());
        let calls = AtomicUsize::new(0);
        for _ in 0..2 {
            assert!(load_with(&cache, &source, ICON_SIZE, |_, size| {
                calls.fetch_add(1, Ordering::SeqCst);
                Some(rgba(size))
            })
            .is_some());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(cache.lock().unwrap().values.is_empty());
    }

    #[test]
    fn fifo_cache_is_bounded_and_includes_failures() {
        let mut cache = IconCache::default();
        for index in 0..=MAX_CACHE_ENTRIES {
            cache.insert(
                CacheKey {
                    source: PathBuf::from(format!("source-{index}")),
                    modified: SystemTime::UNIX_EPOCH,
                    source_len: index as u64,
                    output_size: ICON_SIZE,
                },
                None,
            );
        }
        assert_eq!(cache.values.len(), MAX_CACHE_ENTRIES);
        assert!(!cache
            .values
            .keys()
            .any(|key| key.source == Path::new("source-0")));
    }

    #[test]
    fn icon_only_exists_in_runtime_view() {
        let persisted = EditorInfo {
            name: "Editor".to_string(),
            installed: true,
            launch: Some(EditorLaunch::Executable {
                path: "/editor".to_string(),
                args: vec!["--wait".to_string()],
                working_directory: None,
            }),
            icon_source: Some("/editor".to_string()),
        };
        let cache = EditorCache::from([("editor".to_string(), persisted)]);
        let views = project_auto_editors_with(&cache, |_| {
            Some("data:image/png;base64,runtime-only".to_string())
        });

        assert!(views["editor"].icon.is_some());
        assert!(!serde_json::to_string(&cache)
            .unwrap()
            .contains("data:image"));
        assert!(!views["editor"].can_edit_args);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_extracts_current_executable_icon() {
        clear_cache();
        let executable = std::env::current_exe().unwrap();
        let data_url = icon_data_url(&executable).expect("current exe should have a shell icon");
        let (info, pixels) = decode(&data_url);
        assert_eq!((info.width, info.height), (ICON_SIZE, ICON_SIZE));
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_icon_extraction_does_not_leak_gdi_objects() {
        use windows::Win32::System::Threading::{
            GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
        };

        let executable = std::env::current_exe().unwrap();
        assert!(extract_icon_rgba(&executable, ICON_SIZE).is_some());
        let before = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        for _ in 0..128 {
            assert!(extract_icon_rgba(&executable, ICON_SIZE).is_some());
        }
        let after = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        assert!(
            after <= before.saturating_add(2),
            "GDI objects grew from {before} to {after}"
        );
    }
}
