use std::ffi::CString;
use std::{mem, slice};
use std::path::PathBuf;
use std::os::raw::{c_void, c_char};
use std::sync::Arc;
use std::collections::HashMap;
use gleam::gl;

use webrender_traits::*;
use webrender::renderer::{Renderer, RendererOptions};
use webrender::renderer::{ExternalImage, ExternalImageHandler, ExternalImageSource};
use webrender::{ApiRecordingReceiver, BinaryRecorder};
use app_units::Au;
use euclid::{TypedPoint2D, TypedSize2D, TypedRect, TypedMatrix4D, SideOffsets2D};

extern crate webrender_traits;

// Enables binary recording that can be used with `wrench replay`
// Outputs a wr-record-*.bin file for each window that is shown
// Note: wrench will panic if external images are used, they can
// be disabled in WebRenderBridgeParent::ProcessWebRenderCommands
// by commenting out the path that adds an external image ID
static ENABLE_RECORDING: bool = false;

type WrAPI = RenderApi;
type WrAuxiliaryListsDescriptor = AuxiliaryListsDescriptor;
type WrBorderStyle = BorderStyle;
type WrBoxShadowClipMode = BoxShadowClipMode;
type WrBuiltDisplayListDescriptor = BuiltDisplayListDescriptor;
type WrImageFormat = ImageFormat;
type WrImageRendering = ImageRendering;
type WrMixBlendMode = MixBlendMode;
type WrRenderer = Renderer;
type WrSideOffsets2Du32 = WrSideOffsets2D<u32>;
type WrSideOffsets2Df32 = WrSideOffsets2D<f32>;

/// cbindgen:field-names=[mHandle]
/// cbindgen:struct-gen-op-lt=true
/// cbindgen:struct-gen-op-lte=true
type WrEpoch = Epoch;
/// cbindgen:field-names=[mHandle]
/// cbindgen:struct-gen-op-lt=true
/// cbindgen:struct-gen-op-lte=true
type WrIdNamespace = IdNamespace;

/// cbindgen:field-names=[mNamespace, mHandle]
type WrPipelineId = PipelineId;
/// cbindgen:field-names=[mNamespace, mHandle]
type WrImageKey = ImageKey;
/// cbindgen:field-names=[mNamespace, mHandle]
type WrFontKey = FontKey;

/// cbindgen:field-names=[mHandle]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct WrExternalImageId(pub u64);

impl Into<ExternalImageId> for WrExternalImageId {
    fn into(self) -> ExternalImageId {
        ExternalImageId(self.0)
    }
}
impl Into<WrExternalImageId> for ExternalImageId {
    fn into(self) -> WrExternalImageId {
        WrExternalImageId(self.0)
    }
}

const GL_FORMAT_BGRA_GL: gl::GLuint = gl::BGRA;
const GL_FORMAT_BGRA_GLES: gl::GLuint = gl::BGRA_EXT;

fn get_gl_format_bgra(gl: &gl::Gl) -> gl::GLuint {
    match gl.get_type() {
        gl::GlType::Gl => GL_FORMAT_BGRA_GL,
        gl::GlType::Gles => GL_FORMAT_BGRA_GLES,
    }
}

#[repr(C)]
pub struct WrByteSlice {
    buffer: *const u8,
    len: usize,
}

impl WrByteSlice {
    pub fn new(slice: &[u8]) -> WrByteSlice {
        WrByteSlice {
            buffer: &slice[0],
            len: slice.len(),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.buffer, self.len) }
    }
}

#[repr(C)]
pub struct MutByteSlice {
    buffer: *mut u8,
    len: usize,
}

impl MutByteSlice {
    pub fn new(slice: &mut [u8]) -> MutByteSlice {
        let len = slice.len();
        MutByteSlice {
            buffer: &mut slice[0],
            len: len,
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.buffer, self.len) }
    }
}

#[repr(u32)]
pub enum WrGradientExtendMode {
    Clamp,
    Repeat,
}

impl Into<ExtendMode> for WrGradientExtendMode {
    fn into(self) -> ExtendMode {
        match self {
            WrGradientExtendMode::Clamp => ExtendMode::Clamp,
            WrGradientExtendMode::Repeat => ExtendMode::Repeat,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WrItemRange {
    start: usize,
    length: usize,
}

impl Into<ItemRange> for WrItemRange {
    fn into(self) -> ItemRange {
        ItemRange {
            start: self.start,
            length: self.length,
        }
    }
}
impl From<ItemRange> for WrItemRange {
    fn from(item_range: ItemRange) -> Self {
        WrItemRange {
            start: item_range.start,
            length: item_range.length,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrPoint {
    x: f32,
    y: f32,
}

impl<U> Into<TypedPoint2D<f32, U>> for WrPoint {
    fn into(self) -> TypedPoint2D<f32, U> {
        TypedPoint2D::new(self.x, self.y)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrSize {
    width: f32,
    height: f32,
}

impl<U> Into<TypedSize2D<f32, U>> for WrSize {
    fn into(self) -> TypedSize2D<f32, U> {
        TypedSize2D::new(self.width, self.height)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl<U> Into<TypedRect<f32, U>> for WrRect {
    fn into(self) -> TypedRect<f32, U> {
        TypedRect::new(TypedPoint2D::new(self.x, self.y),
                       TypedSize2D::new(self.width, self.height))
    }
}
impl<U> From<TypedRect<f32, U>> for WrRect {
    fn from(rect: TypedRect<f32, U>) -> Self {
        WrRect {
            x: rect.origin.x,
            y: rect.origin.y,
            width: rect.size.width,
            height: rect.size.height,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrMatrix {
    values: [f32; 16],
}

impl<'a, U, E> Into<TypedMatrix4D<f32, U, E>> for &'a WrMatrix {
    fn into(self) -> TypedMatrix4D<f32, U, E> {
        TypedMatrix4D::row_major(self.values[0],
                                 self.values[1],
                                 self.values[2],
                                 self.values[3],
                                 self.values[4],
                                 self.values[5],
                                 self.values[6],
                                 self.values[7],
                                 self.values[8],
                                 self.values[9],
                                 self.values[10],
                                 self.values[11],
                                 self.values[12],
                                 self.values[13],
                                 self.values[14],
                                 self.values[15])
    }
}
impl<U, E> Into<TypedMatrix4D<f32, U, E>> for WrMatrix {
    fn into(self) -> TypedMatrix4D<f32, U, E> {
        TypedMatrix4D::row_major(self.values[0],
                                 self.values[1],
                                 self.values[2],
                                 self.values[3],
                                 self.values[4],
                                 self.values[5],
                                 self.values[6],
                                 self.values[7],
                                 self.values[8],
                                 self.values[9],
                                 self.values[10],
                                 self.values[11],
                                 self.values[12],
                                 self.values[13],
                                 self.values[14],
                                 self.values[15])
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrColor {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Into<ColorF> for WrColor {
    fn into(self) -> ColorF {
        ColorF::new(self.r, self.g, self.b, self.a)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrGlyphInstance {
    index: u32,
    point: WrPoint,
}

impl<'a> Into<GlyphInstance> for &'a WrGlyphInstance {
    fn into(self) -> GlyphInstance {
        GlyphInstance {
            index: self.index,
            point: self.point.into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrGradientStop {
    offset: f32,
    color: WrColor,
}

impl<'a> Into<GradientStop> for &'a WrGradientStop {
    fn into(self) -> GradientStop {
        GradientStop {
            offset: self.offset,
            color: self.color.into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrBorderSide {
    color: WrColor,
    style: WrBorderStyle,
}

impl Into<BorderSide> for WrBorderSide {
    fn into(self) -> BorderSide {
        BorderSide {
            color: self.color.into(),
            style: self.style,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrBorderRadius {
    pub top_left: WrSize,
    pub top_right: WrSize,
    pub bottom_left: WrSize,
    pub bottom_right: WrSize,
}

impl Into<BorderRadius> for WrBorderRadius {
    fn into(self) -> BorderRadius {
        BorderRadius {
            top_left: self.top_left.into(),
            top_right: self.top_right.into(),
            bottom_left: self.bottom_left.into(),
            bottom_right: self.bottom_right.into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrBorderWidths {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Into<BorderWidths> for WrBorderWidths {
    fn into(self) -> BorderWidths {
        BorderWidths {
            left: self.left,
            top: self.top,
            right: self.right,
            bottom: self.bottom,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrNinePatchDescriptor {
    width: u32,
    height: u32,
    slice: WrSideOffsets2Du32,
}

impl Into<NinePatchDescriptor> for WrNinePatchDescriptor {
    fn into(self) -> NinePatchDescriptor {
        NinePatchDescriptor {
            width: self.width,
            height: self.height,
            slice: self.slice.into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrSideOffsets2D<T> {
    top: T,
    right: T,
    bottom: T,
    left: T,
}

impl<T: Copy> Into<SideOffsets2D<T>> for WrSideOffsets2D<T> {
    fn into(self) -> SideOffsets2D<T> {
        SideOffsets2D::new(self.top, self.right, self.bottom, self.left)
    }
}

#[repr(u32)]
pub enum WrRepeatMode {
    Stretch,
    Repeat,
    Round,
    Space,
}

impl Into<RepeatMode> for WrRepeatMode {
    fn into(self) -> RepeatMode {
        match self {
            WrRepeatMode::Stretch => RepeatMode::Stretch,
            WrRepeatMode::Repeat => RepeatMode::Repeat,
            WrRepeatMode::Round => RepeatMode::Round,
            WrRepeatMode::Space => RepeatMode::Space,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrImageMask {
    image: WrImageKey,
    rect: WrRect,
    repeat: bool,
}

impl Into<ImageMask> for WrImageMask {
    fn into(self) -> ImageMask {
        ImageMask {
            image: self.image,
            rect: self.rect.into(),
            repeat: self.repeat,
        }
    }
}
impl<'a> Into<ImageMask> for &'a WrImageMask {
    fn into(self) -> ImageMask {
        ImageMask {
            image: self.image,
            rect: self.rect.into(),
            repeat: self.repeat,
        }
    }
}
impl From<ImageMask> for WrImageMask {
    fn from(image_mask: ImageMask) -> Self {
        WrImageMask {
            image: image_mask.image,
            rect: image_mask.rect.into(),
            repeat: image_mask.repeat,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrComplexClipRegion {
    rect: WrRect,
    radii: WrBorderRadius,
}

impl<'a> Into<ComplexClipRegion> for &'a WrComplexClipRegion {
    fn into(self) -> ComplexClipRegion {
        ComplexClipRegion {
            rect: self.rect.into(),
            radii: self.radii.into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WrClipRegion {
    main: WrRect,
    complex: WrItemRange,
    image_mask: WrImageMask,
    has_image_mask: bool,
}

impl Into<ClipRegion> for WrClipRegion {
    fn into(self) -> ClipRegion {
        ClipRegion {
            main: self.main.into(),
            complex: self.complex.into(),
            image_mask: if self.has_image_mask {
                Some(self.image_mask.into())
            } else {
                None
            },
        }
    }
}
impl From<ClipRegion> for WrClipRegion {
    fn from(clip_region: ClipRegion) -> Self {
        if let Some(image_mask) = clip_region.image_mask {
            WrClipRegion {
                main: clip_region.main.into(),
                complex: clip_region.complex.into(),
                image_mask: image_mask.into(),
                has_image_mask: true,
            }
        } else {
            let blank = WrImageMask {
                image: ImageKey(0, 0),
                rect: WrRect {
                    x: 0f32,
                    y: 0f32,
                    width: 0f32,
                    height: 0f32,
                },
                repeat: false,
            };

            WrClipRegion {
                main: clip_region.main.into(),
                complex: clip_region.complex.into(),
                image_mask: blank,
                has_image_mask: false,
            }
        }
    }
}

#[repr(u32)]
#[allow(dead_code)]
enum WrExternalImageType {
    NativeTexture,
    RawData,
}

#[repr(C)]
struct WrExternalImage {
    image_type: WrExternalImageType,

    // external texture handle
    handle: u32,
    // external texture coordinate
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,

    // external image buffer
    buff: *const u8,
    size: usize,
}

type LockExternalImageCallback = fn(*mut c_void, WrExternalImageId) -> WrExternalImage;
type UnlockExternalImageCallback = fn(*mut c_void, WrExternalImageId);
type ReleaseExternalImageCallback = fn(*mut c_void, WrExternalImageId);

#[repr(C)]
pub struct WrExternalImageHandler {
    external_image_obj: *mut c_void,
    lock_func: LockExternalImageCallback,
    unlock_func: UnlockExternalImageCallback,
    release_func: ReleaseExternalImageCallback,
}

impl ExternalImageHandler for WrExternalImageHandler {
    fn lock(&mut self,
            id: ExternalImageId)
            -> ExternalImage {
        let image = (self.lock_func)(self.external_image_obj, id.into());

        match image.image_type {
            WrExternalImageType::NativeTexture => {
                ExternalImage {
                    u0: image.u0,
                    v0: image.v0,
                    u1: image.u1,
                    v1: image.v1,
                    source: ExternalImageSource::NativeTexture(image.handle),
                }
            },
            WrExternalImageType::RawData => {
                ExternalImage {
                    u0: image.u0,
                    v0: image.v0,
                    u1: image.u1,
                    v1: image.v1,
                    source: ExternalImageSource::RawData(unsafe { slice::from_raw_parts(image.buff, image.size) }),
                }
            },
        }
    }

    fn unlock(&mut self,
              id: ExternalImageId) {
        (self.unlock_func)(self.external_image_obj, id.into());
    }

    fn release(&mut self,
               id: ExternalImageId) {
        (self.release_func)(self.external_image_obj, id.into());
    }
}

/// cbindgen:field-names=[mHandle]
/// cbindgen:struct-gen-op-lt=true
/// cbindgen:struct-gen-op-lte=true
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WrWindowId(u64);

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WrImageDescriptor {
    pub format: WrImageFormat,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub is_opaque: bool,
}

impl<'a> Into<ImageDescriptor> for &'a WrImageDescriptor {
    fn into(self) -> ImageDescriptor {
        ImageDescriptor {
            width: self.width,
            height: self.height,
            stride: if self.stride != 0 {
                Some(self.stride)
            } else {
                None
            },
            format: self.format,
            is_opaque: self.is_opaque,
            offset: 0,
        }
    }
}

#[repr(C)]
pub struct WrVecU8 {
    data: *mut u8,
    length: usize,
    capacity: usize,
}

impl WrVecU8 {
    fn to_vec(self) -> Vec<u8> {
        unsafe { Vec::from_raw_parts(self.data, self.length, self.capacity) }
    }
    fn from_vec(mut v: Vec<u8>) -> WrVecU8 {
        let w = WrVecU8 {
            data: v.as_mut_ptr(),
            length: v.len(),
            capacity: v.capacity(),
        };
        mem::forget(v);
        w
    }
}

#[no_mangle]
pub extern "C" fn wr_vec_u8_free(v: WrVecU8) {
    v.to_vec();
}

#[repr(C)]
#[derive(Debug)]
pub struct WrTransformProperty {
    pub id: u64,
    pub transform: WrMatrix,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WrOpacityProperty {
    pub id: u64,
    pub opacity: f32,
}

fn get_proc_address(glcontext_ptr: *mut c_void,
                    name: &str)
                    -> *const c_void {

    extern "C" {
        fn get_proc_address_from_glcontext(glcontext_ptr: *mut c_void,
                                           procname: *const c_char)
                                           -> *const c_void;
    }

    let symbol_name = CString::new(name).unwrap();
    let symbol = unsafe { get_proc_address_from_glcontext(glcontext_ptr, symbol_name.as_ptr()) };

    // For now panic, not sure we should be though or if we can recover
    if symbol.is_null() {
        // XXX Bug 1322949 Make whitelist for extensions
        println!("Could not find symbol {:?} by glcontext", symbol_name);
    }

    symbol as *const _
}

extern "C" {
    fn is_in_compositor_thread() -> bool;
    fn is_in_render_thread() -> bool;
    fn is_in_main_thread() -> bool;
    fn is_glcontext_egl(glcontext_ptr: *mut c_void) -> bool;
    fn gfx_critical_note(msg: *const c_char);
}

struct CppNotifier {
    window_id: WrWindowId,
}

unsafe impl Send for CppNotifier {}

extern "C" {
    fn wr_notifier_new_frame_ready(window_id: WrWindowId);
    fn wr_notifier_new_scroll_frame_ready(window_id: WrWindowId,
                                          composite_needed: bool);
    fn wr_notifier_external_event(window_id: WrWindowId,
                                  raw_event: usize);
}

impl webrender_traits::RenderNotifier for CppNotifier {
    fn new_frame_ready(&mut self) {
        unsafe {
            wr_notifier_new_frame_ready(self.window_id);
        }
    }

    fn new_scroll_frame_ready(&mut self,
                              composite_needed: bool) {
        unsafe {
            wr_notifier_new_scroll_frame_ready(self.window_id, composite_needed);
        }
    }

    fn external_event(&mut self,
                      event: ExternalEvent) {
        unsafe {
            wr_notifier_external_event(self.window_id, event.unwrap());
        }
    }
}

#[no_mangle]
pub extern "C" fn wr_renderer_set_external_image_handler(renderer: &mut WrRenderer,
                                                         external_image_handler: *mut WrExternalImageHandler) {
    if !external_image_handler.is_null() {
        renderer.set_external_image_handler(Box::new(unsafe {
                                                         WrExternalImageHandler {
                                                             external_image_obj:
                                                                 (*external_image_handler).external_image_obj,
                                                             lock_func: (*external_image_handler).lock_func,
                                                             unlock_func: (*external_image_handler).unlock_func,
                                                             release_func: (*external_image_handler).release_func,
                                                         }
                                                     }));
    }
}

#[no_mangle]
pub extern "C" fn wr_renderer_update(renderer: &mut WrRenderer) {
    renderer.update();
}

#[no_mangle]
pub extern "C" fn wr_renderer_render(renderer: &mut WrRenderer,
                                     width: u32,
                                     height: u32) {
    renderer.render(DeviceUintSize::new(width, height));
}

// Call wr_renderer_render() before calling this function.
#[no_mangle]
pub unsafe extern "C" fn wr_renderer_readback(renderer: &mut WrRenderer,
                                              width: u32,
                                              height: u32,
                                              dst_buffer: *mut u8,
                                              buffer_size: usize) {
    assert!(is_in_render_thread());

    renderer.gl().flush();

    let mut slice = slice::from_raw_parts_mut(dst_buffer, buffer_size);
    renderer.gl()
            .read_pixels_into_buffer(0,
                                     0,
                                     width as gl::GLsizei,
                                     height as gl::GLsizei,
                                     get_gl_format_bgra(renderer.gl()),
                                     gl::UNSIGNED_BYTE,
                                     slice);
}

#[no_mangle]
pub extern "C" fn wr_renderer_set_profiler_enabled(renderer: &mut WrRenderer,
                                                   enabled: bool) {
    renderer.set_profiler_enabled(enabled);
}

#[no_mangle]
pub extern "C" fn wr_renderer_current_epoch(renderer: &mut WrRenderer,
                                            pipeline_id: WrPipelineId,
                                            out_epoch: &mut WrEpoch)
                                            -> bool {
    if let Some(epoch) = renderer.current_epoch(pipeline_id) {
        *out_epoch = epoch;
        return true;
    }
    return false;
}

/// cbindgen:function-postfix=WR_DESTRUCTOR_SAFE_FUNC
#[no_mangle]
pub unsafe extern "C" fn wr_renderer_delete(renderer: *mut WrRenderer) {
    Box::from_raw(renderer);
}

pub struct WrRenderedEpochs {
    data: Vec<(WrPipelineId, WrEpoch)>,
}

#[no_mangle]
pub unsafe extern "C" fn wr_renderer_flush_rendered_epochs(renderer: &mut WrRenderer) -> *mut WrRenderedEpochs {
    let map = renderer.flush_rendered_epochs();
    let pipeline_epochs = Box::new(WrRenderedEpochs {
                                       data: map.into_iter().collect(),
                                   });
    return Box::into_raw(pipeline_epochs);
}

#[no_mangle]
pub unsafe extern "C" fn wr_rendered_epochs_next(pipeline_epochs: &mut WrRenderedEpochs,
                                                 out_pipeline: &mut WrPipelineId,
                                                 out_epoch: &mut WrEpoch)
                                                 -> bool {
    if let Some((pipeline, epoch)) = pipeline_epochs.data.pop() {
        *out_pipeline = pipeline;
        *out_epoch = epoch;
        return true;
    }
    return false;
}

/// cbindgen:function-postfix=WR_DESTRUCTOR_SAFE_FUNC
#[no_mangle]
pub unsafe extern "C" fn wr_rendered_epochs_delete(pipeline_epochs: *mut WrRenderedEpochs) {
    Box::from_raw(pipeline_epochs);
}

// Call MakeCurrent before this.
#[no_mangle]
pub extern "C" fn wr_window_new(window_id: WrWindowId,
                                window_width: u32,
                                window_height: u32,
                                gl_context: *mut c_void,
                                enable_profiler: bool,
                                out_api: &mut *mut WrAPI,
                                out_renderer: &mut *mut WrRenderer)
                                -> bool {
    assert!(unsafe { is_in_render_thread() });

    let recorder: Option<Box<ApiRecordingReceiver>> = if ENABLE_RECORDING {
        let name = format!("wr-record-{}.bin", window_id.0);
        Some(Box::new(BinaryRecorder::new(&PathBuf::from(name))))
    } else {
        None
    };

    let gl;
    if unsafe { is_glcontext_egl(gl_context) } {
        gl = unsafe { gl::GlesFns::load_with(|symbol| get_proc_address(gl_context, symbol)) };
    } else {
        gl = unsafe { gl::GlFns::load_with(|symbol| get_proc_address(gl_context, symbol)) };
    }
    gl.clear_color(0.3, 0.0, 0.0, 1.0);

    let version = gl.get_string(gl::VERSION);

    println!("WebRender - OpenGL version new {}", version);

    let opts = RendererOptions {
        enable_aa: true,
        enable_subpixel_aa: true,
        enable_profiler: enable_profiler,
        recorder: recorder,
        blob_image_renderer: Some(Box::new(Moz2dImageRenderer::new())),
        ..Default::default()
    };

    let window_size = DeviceUintSize::new(window_width, window_height);
    let (renderer, sender) = match WrRenderer::new(gl, opts, window_size) {
        Ok((renderer, sender)) => (renderer, sender),
        Err(e) => {
            println!(" Failed to create a WrRenderer: {:?}", e);
            let msg = CString::new(format!("wr_window_new: {:?}", e)).unwrap();
            unsafe {
                gfx_critical_note(msg.as_ptr());
            }
            return false;
        },
    };

    renderer.set_render_notifier(Box::new(CppNotifier {
                                              window_id: window_id,
                                          }));

    *out_api = Box::into_raw(Box::new(sender.create_api()));
    *out_renderer = Box::into_raw(Box::new(renderer));

    return true;
}

#[no_mangle]
pub unsafe extern "C" fn wr_api_delete(api: *mut WrAPI) {
    let api = Box::from_raw(api);
    api.shut_down();
}

#[no_mangle]
pub extern "C" fn wr_api_add_image(api: &mut WrAPI,
                                   image_key: WrImageKey,
                                   descriptor: &WrImageDescriptor,
                                   bytes: WrByteSlice) {
    assert!(unsafe { is_in_compositor_thread() });
    let copied_bytes = bytes.as_slice().to_owned();
    api.add_image(image_key,
                  descriptor.into(),
                  ImageData::new(copied_bytes),
                  None);
}

#[no_mangle]
pub extern "C" fn wr_api_add_blob_image(api: &mut WrAPI,
                                        image_key: WrImageKey,
                                        descriptor: &WrImageDescriptor,
                                        bytes: WrByteSlice) {
    assert!(unsafe { is_in_compositor_thread() });
    let copied_bytes = bytes.as_slice().to_owned();
    api.add_image(image_key,
                  descriptor.into(),
                  ImageData::new_blob_image(copied_bytes),
                  None);
}

#[no_mangle]
pub extern "C" fn wr_api_add_external_image_handle(api: &mut WrAPI,
                                                   image_key: WrImageKey,
                                                   descriptor: &WrImageDescriptor,
                                                   external_image_id: u64) {
    assert!(unsafe { is_in_compositor_thread() });
    api.add_image(image_key,
                  descriptor.into(),
                  ImageData::External(ExternalImageData {
                                          id: ExternalImageId(external_image_id),
                                          image_type: ExternalImageType::Texture2DHandle,
                                      }),
                  None);
}

#[no_mangle]
pub extern "C" fn wr_api_add_external_image_buffer(api: &mut WrAPI,
                                                   image_key: WrImageKey,
                                                   descriptor: &WrImageDescriptor,
                                                   external_image_id: u64) {
    assert!(unsafe { is_in_compositor_thread() });
    api.add_image(image_key,
                  descriptor.into(),
                  ImageData::External(ExternalImageData {
                                          id: ExternalImageId(external_image_id),
                                          image_type: ExternalImageType::ExternalBuffer,
                                      }),
                  None);
}

#[no_mangle]
pub extern "C" fn wr_api_update_image(api: &mut WrAPI,
                                      key: WrImageKey,
                                      descriptor: &WrImageDescriptor,
                                      bytes: WrByteSlice) {
    assert!(unsafe { is_in_compositor_thread() });
    let copied_bytes = bytes.as_slice().to_owned();

    api.update_image(key, descriptor.into(), ImageData::new(copied_bytes), None);
}

#[no_mangle]
pub extern "C" fn wr_api_delete_image(api: &mut WrAPI,
                                      key: WrImageKey) {
    assert!(unsafe { is_in_compositor_thread() });
    api.delete_image(key)
}

#[no_mangle]
pub extern "C" fn wr_api_set_root_pipeline(api: &mut WrAPI,
                                           pipeline_id: WrPipelineId) {
    api.set_root_pipeline(pipeline_id);
    api.generate_frame(None);
}

#[no_mangle]
pub extern "C" fn wr_api_set_window_parameters(api: &mut WrAPI,
                                               width: i32,
                                               height: i32) {
    let size = DeviceUintSize::new(width as u32, height as u32);
    api.set_window_parameters(size, DeviceUintRect::new(DeviceUintPoint::new(0, 0), size));
}

#[no_mangle]
pub unsafe extern "C" fn wr_api_set_root_display_list(api: &mut WrAPI,
                                                      epoch: WrEpoch,
                                                      viewport_width: f32,
                                                      viewport_height: f32,
                                                      pipeline_id: WrPipelineId,
                                                      dl_descriptor: WrBuiltDisplayListDescriptor,
                                                      dl_data: *mut u8,
                                                      dl_size: usize,
                                                      aux_descriptor: WrAuxiliaryListsDescriptor,
                                                      aux_data: *mut u8,
                                                      aux_size: usize) {
    let root_background_color = ColorF::new(0.3, 0.0, 0.0, 1.0);
    // See the documentation of set_display_list in api.rs. I don't think
    // it makes a difference in gecko at the moment(until APZ is figured out)
    // but I suppose it is a good default.
    let preserve_frame_state = true;

    let dl_slice = slice::from_raw_parts(dl_data, dl_size);
    let mut dl_vec = Vec::new();
    // XXX: see if we can get rid of the copy here
    dl_vec.extend_from_slice(dl_slice);
    let dl = BuiltDisplayList::from_data(dl_vec, dl_descriptor);

    let aux_slice = slice::from_raw_parts(aux_data, aux_size);
    let mut aux_vec = Vec::new();
    // XXX: see if we can get rid of the copy here
    aux_vec.extend_from_slice(aux_slice);
    let aux = AuxiliaryLists::from_data(aux_vec, aux_descriptor);

    api.set_display_list(Some(root_background_color),
                         epoch,
                         LayoutSize::new(viewport_width, viewport_height),
                         (pipeline_id, dl, aux),
                         preserve_frame_state);
}

#[no_mangle]
pub unsafe extern "C" fn wr_api_clear_root_display_list(api: &mut WrAPI,
                                                        epoch: WrEpoch,
                                                        pipeline_id: WrPipelineId) {
    let root_background_color = ColorF::new(0.3, 0.0, 0.0, 1.0);
    let preserve_frame_state = true;
    let frame_builder = WebRenderFrameBuilder::new(pipeline_id);

    api.set_display_list(Some(root_background_color),
                         epoch,
                         LayoutSize::new(0.0, 0.0),
                         frame_builder.dl_builder.finalize(),
                         preserve_frame_state);
}

#[no_mangle]
pub extern "C" fn wr_api_generate_frame(api: &mut WrAPI) {
    api.generate_frame(None);
}

#[no_mangle]
pub extern "C" fn wr_api_generate_frame_with_properties(api: &mut WrAPI,
                                                        opacity_array: *const WrOpacityProperty,
                                                        opacity_count: usize,
                                                        transform_array: *const WrTransformProperty,
                                                        transform_count: usize) {
    let mut properties = DynamicProperties {
        transforms: Vec::new(),
        floats: Vec::new(),
    };

    if transform_count > 0 {
        let transform_slice = unsafe { slice::from_raw_parts(transform_array, transform_count) };

        for element in transform_slice.iter() {
            let prop = PropertyValue {
                key: PropertyBindingKey::new(element.id),
                value: element.transform.into(),
            };

            properties.transforms.push(prop);
        }
    }

    if opacity_count > 0 {
        let opacity_slice = unsafe { slice::from_raw_parts(opacity_array, opacity_count) };

        for element in opacity_slice.iter() {
            let prop = PropertyValue {
                key: PropertyBindingKey::new(element.id),
                value: element.opacity,
            };
            properties.floats.push(prop);
        }
    }

    api.generate_frame(Some(properties));
}

/// cbindgen:function-postfix=WR_DESTRUCTOR_SAFE_FUNC
#[no_mangle]
pub extern "C" fn wr_api_send_external_event(api: &mut WrAPI,
                                             evt: usize) {
    assert!(unsafe { !is_in_render_thread() });

    api.send_external_event(ExternalEvent::from_raw(evt));
}

#[no_mangle]
pub extern "C" fn wr_api_add_raw_font(api: &mut WrAPI,
                                      key: WrFontKey,
                                      font_buffer: *mut u8,
                                      buffer_size: usize) {
    assert!(unsafe { is_in_compositor_thread() });

    let font_slice = unsafe { slice::from_raw_parts(font_buffer, buffer_size as usize) };
    let mut font_vector = Vec::new();
    font_vector.extend_from_slice(font_slice);

    api.add_raw_font(key, font_vector);
}

#[no_mangle]
pub extern "C" fn wr_api_delete_font(api: &mut WrAPI,
                                     key: WrFontKey) {
    assert!(unsafe { is_in_compositor_thread() });
    api.delete_font(key);
}

#[no_mangle]
pub unsafe extern "C" fn wr_api_get_namespace(api: &mut WrAPI) -> WrIdNamespace {
    api.id_namespace
}

// RenderThread WIP notes:
// In order to separate the compositor thread (or ipc receiver) and the render
// thread, some of the logic below needs to be rewritten. In particular
// the WrWindowState and Notifier implementations aren't designed to work with
// a separate render thread.
// As part of that I am moving the bindings closer to WebRender's API boundary,
// and moving more of the logic in C++ land.
// This work is tracked by bug 1328602.
//
// See RenderThread.h for some notes about how the pieces fit together.

pub struct WebRenderFrameBuilder {
    pub root_pipeline_id: WrPipelineId,
    pub dl_builder: webrender_traits::DisplayListBuilder,
}

impl WebRenderFrameBuilder {
    pub fn new(root_pipeline_id: WrPipelineId) -> WebRenderFrameBuilder {
        WebRenderFrameBuilder {
            root_pipeline_id: root_pipeline_id,
            dl_builder: webrender_traits::DisplayListBuilder::new(root_pipeline_id),
        }
    }
}

pub struct WrState {
    pipeline_id: WrPipelineId,
    z_index: i32,
    frame_builder: WebRenderFrameBuilder,
}

#[no_mangle]
pub extern "C" fn wr_state_new(pipeline_id: WrPipelineId) -> *mut WrState {
    assert!(unsafe { is_in_main_thread() });

    let state = Box::new(WrState {
                             pipeline_id: pipeline_id,
                             z_index: 0,
                             frame_builder: WebRenderFrameBuilder::new(pipeline_id),
                         });

    Box::into_raw(state)
}

#[no_mangle]
pub extern "C" fn wr_state_delete(state: *mut WrState) {
    assert!(unsafe { is_in_main_thread() });

    unsafe {
        Box::from_raw(state);
    }
}

#[no_mangle]
pub extern "C" fn wr_dp_begin(state: &mut WrState,
                              width: u32,
                              height: u32) {
    assert!(unsafe { is_in_main_thread() });
    state.frame_builder.dl_builder.list.clear();
    state.z_index = 0;

    let bounds = LayoutRect::new(LayoutPoint::new(0.0, 0.0),
                                 LayoutSize::new(width as f32, height as f32));

    state.frame_builder
         .dl_builder
         .push_stacking_context(webrender_traits::ScrollPolicy::Scrollable,
                                bounds,
                                0,
                                None,
                                TransformStyle::Flat,
                                None,
                                MixBlendMode::Normal,
                                Vec::new());
}

#[no_mangle]
pub extern "C" fn wr_dp_end(state: &mut WrState) {
    assert!(unsafe { is_in_main_thread() });
    state.frame_builder.dl_builder.pop_stacking_context();
}

#[no_mangle]
pub extern "C" fn wr_dp_new_clip_region(state: &mut WrState,
                                        main: WrRect,
                                        complex: *const WrComplexClipRegion,
                                        complex_count: usize,
                                        image_mask: *const WrImageMask)
                                        -> WrClipRegion {
    assert!(unsafe { is_in_main_thread() });

    let main = main.into();
    let complex_slice = unsafe { slice::from_raw_parts(complex, complex_count) };
    let complex_vector = complex_slice.iter().map(|x| x.into()).collect();
    let mask = unsafe { image_mask.as_ref() }.map(|x| x.into());

    let clip_region = state.frame_builder.dl_builder.new_clip_region(&main, complex_vector, mask);

    clip_region.into()
}

#[no_mangle]
pub extern "C" fn wr_dp_push_stacking_context(state: &mut WrState,
                                              bounds: WrRect,
                                              animation_id: u64,
                                              opacity: *const f32,
                                              transform: *const WrMatrix,
                                              mix_blend_mode: WrMixBlendMode) {
    assert!(unsafe { is_in_main_thread() });
    state.z_index += 1;

    let bounds = bounds.into();

    let mut filters: Vec<FilterOp> = Vec::new();
    let opacity = unsafe { opacity.as_ref() };
    if let Some(opacity) = opacity {
        if *opacity < 1.0 {
            filters.push(FilterOp::Opacity(PropertyBinding::Value(*opacity)));
        }
    } else {
        filters.push(FilterOp::Opacity(PropertyBinding::Binding(PropertyBindingKey::new(animation_id))));
    }

    let transform = unsafe { transform.as_ref() };
    let transform_binding = match transform {
        Some(transform) => PropertyBinding::Value(transform.into()),
        None => PropertyBinding::Binding(PropertyBindingKey::new(animation_id)),
    };

    state.frame_builder
         .dl_builder
         .push_stacking_context(webrender_traits::ScrollPolicy::Scrollable,
                                bounds,
                                state.z_index,
                                Some(transform_binding),
                                TransformStyle::Flat,
                                None,
                                mix_blend_mode,
                                filters);
}

#[no_mangle]
pub extern "C" fn wr_dp_pop_stacking_context(state: &mut WrState) {
    assert!(unsafe { is_in_main_thread() });
    state.frame_builder.dl_builder.pop_stacking_context();
}

#[no_mangle]
pub extern "C" fn wr_dp_push_scroll_layer(state: &mut WrState,
                                          content_rect: WrRect,
                                          clip_rect: WrRect,
                                          mask: *const WrImageMask) {
    let content_rect = content_rect.into();
    let clip_rect = clip_rect.into();
    let mask = unsafe { mask.as_ref() }.map(|x| x.into());
    let clip_region = state.frame_builder.dl_builder.new_clip_region(&clip_rect, vec![], mask);
    state.frame_builder.dl_builder.push_clip_node(clip_region, content_rect, None);
}

#[no_mangle]
pub extern "C" fn wr_dp_pop_scroll_layer(state: &mut WrState) {
    assert!(unsafe { is_in_main_thread() });
    state.frame_builder.dl_builder.pop_clip_node();
}

#[no_mangle]
pub extern "C" fn wr_dp_push_iframe(state: &mut WrState,
                                    rect: WrRect,
                                    clip: WrClipRegion,
                                    pipeline_id: WrPipelineId) {
    assert!(unsafe { is_in_main_thread() });

    state.frame_builder.dl_builder.push_iframe(rect.into(), clip.into(), pipeline_id);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_rect(state: &mut WrState,
                                  rect: WrRect,
                                  clip: WrClipRegion,
                                  color: WrColor) {
    assert!(unsafe { is_in_main_thread() });

    state.frame_builder.dl_builder.push_rect(rect.into(), clip.into(), color.into());
}

#[no_mangle]
pub extern "C" fn wr_dp_push_image(state: &mut WrState,
                                   bounds: WrRect,
                                   clip: WrClipRegion,
                                   stretch_size: WrSize,
                                   tile_spacing: WrSize,
                                   image_rendering: WrImageRendering,
                                   key: WrImageKey) {
    assert!(unsafe { is_in_main_thread() });

    let bounds = bounds.into();

    state.frame_builder
         .dl_builder
         .push_image(bounds,
                     clip.into(),
                     stretch_size.into(),
                     tile_spacing.into(),
                     image_rendering,
                     key);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_text(state: &mut WrState,
                                  bounds: WrRect,
                                  clip: WrClipRegion,
                                  color: WrColor,
                                  font_key: WrFontKey,
                                  glyphs: *const WrGlyphInstance,
                                  glyph_count: u32,
                                  glyph_size: f32) {
    assert!(unsafe { is_in_main_thread() });

    let glyph_slice = unsafe { slice::from_raw_parts(glyphs, glyph_count as usize) };
    let glyph_vector: Vec<_> = glyph_slice.iter().map(|x| x.into()).collect();

    let colorf = ColorF::new(color.r, color.g, color.b, color.a);

    let glyph_options = None; // TODO
    state.frame_builder
         .dl_builder
         .push_text(bounds.into(),
                    clip.into(),
                    &glyph_vector,
                    font_key,
                    colorf,
                    Au::from_f32_px(glyph_size),
                    Au::from_px(0),
                    glyph_options);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_border(state: &mut WrState,
                                    rect: WrRect,
                                    clip: WrClipRegion,
                                    widths: WrBorderWidths,
                                    top: WrBorderSide,
                                    right: WrBorderSide,
                                    bottom: WrBorderSide,
                                    left: WrBorderSide,
                                    radius: WrBorderRadius) {
    assert!(unsafe { is_in_main_thread() });

    let border_details = BorderDetails::Normal(NormalBorder {
                                                   left: left.into(),
                                                   right: right.into(),
                                                   top: top.into(),
                                                   bottom: bottom.into(),
                                                   radius: radius.into(),
                                               });
    state.frame_builder
         .dl_builder
         .push_border(rect.into(), clip.into(), widths.into(), border_details);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_border_image(state: &mut WrState,
                                          rect: WrRect,
                                          clip: WrClipRegion,
                                          widths: WrBorderWidths,
                                          image: WrImageKey,
                                          patch: WrNinePatchDescriptor,
                                          outset: WrSideOffsets2Df32,
                                          repeat_horizontal: WrRepeatMode,
                                          repeat_vertical: WrRepeatMode) {
    assert!(unsafe { is_in_main_thread() });
    let border_details =
        BorderDetails::Image(ImageBorder {
                                 image_key: image,
                                 patch: patch.into(),
                                 outset: outset.into(),
                                 repeat_horizontal: repeat_horizontal.into(),
                                 repeat_vertical: repeat_vertical.into(),
                             });
    state.frame_builder
         .dl_builder
         .push_border(rect.into(), clip.into(), widths.into(), border_details);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_border_gradient(state: &mut WrState,
                                             rect: WrRect,
                                             clip: WrClipRegion,
                                             widths: WrBorderWidths,
                                             start_point: WrPoint,
                                             end_point: WrPoint,
                                             stops: *const WrGradientStop,
                                             stops_count: usize,
                                             extend_mode: WrGradientExtendMode,
                                             outset: WrSideOffsets2Df32) {
    assert!(unsafe { is_in_main_thread() });

    let stops_slice = unsafe { slice::from_raw_parts(stops, stops_count) };
    let stops_vector = stops_slice.iter().map(|x| x.into()).collect();

    let border_details = BorderDetails::Gradient(GradientBorder {
                                                     gradient:
                                                         state.frame_builder
                                                              .dl_builder
                                                              .create_gradient(start_point.into(),
                                                                               end_point.into(),
                                                                               stops_vector,
                                                                               extend_mode.into()),
                                                     outset: outset.into(),
                                                 });
    state.frame_builder
         .dl_builder
         .push_border(rect.into(), clip.into(), widths.into(), border_details);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_border_radial_gradient(state: &mut WrState,
                                                    rect: WrRect,
                                                    clip: WrClipRegion,
                                                    widths: WrBorderWidths,
                                                    center: WrPoint,
                                                    radius: WrSize,
                                                    stops: *const WrGradientStop,
                                                    stops_count: usize,
                                                    extend_mode: WrGradientExtendMode,
                                                    outset: WrSideOffsets2Df32) {
    assert!(unsafe { is_in_main_thread() });

    let stops_slice = unsafe { slice::from_raw_parts(stops, stops_count) };
    let stops_vector = stops_slice.iter().map(|x| x.into()).collect();

    let border_details =
        BorderDetails::RadialGradient(RadialGradientBorder {
                                          gradient:
                                              state.frame_builder
                                                   .dl_builder
                                                   .create_radial_gradient(center.into(),
                                                                           radius.into(),
                                                                           stops_vector,
                                                                           extend_mode.into()),
                                          outset: outset.into(),
                                      });
    state.frame_builder
         .dl_builder
         .push_border(rect.into(), clip.into(), widths.into(), border_details);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_linear_gradient(state: &mut WrState,
                                             rect: WrRect,
                                             clip: WrClipRegion,
                                             start_point: WrPoint,
                                             end_point: WrPoint,
                                             stops: *const WrGradientStop,
                                             stops_count: usize,
                                             extend_mode: WrGradientExtendMode,
                                             tile_size: WrSize,
                                             tile_spacing: WrSize) {
    assert!(unsafe { is_in_main_thread() });

    let stops_slice = unsafe { slice::from_raw_parts(stops, stops_count) };
    let stops_vector = stops_slice.iter().map(|x| x.into()).collect();

    let gradient = state.frame_builder
                        .dl_builder
                        .create_gradient(start_point.into(),
                                         end_point.into(),
                                         stops_vector,
                                         extend_mode.into());
    let rect = rect.into();
    let tile_size = tile_size.into();
    let tile_spacing = tile_spacing.into();
    state.frame_builder
         .dl_builder
         .push_gradient(rect, clip.into(), gradient, tile_size, tile_spacing);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_radial_gradient(state: &mut WrState,
                                             rect: WrRect,
                                             clip: WrClipRegion,
                                             center: WrPoint,
                                             radius: WrSize,
                                             stops: *const WrGradientStop,
                                             stops_count: usize,
                                             extend_mode: WrGradientExtendMode,
                                             tile_size: WrSize,
                                             tile_spacing: WrSize) {
    assert!(unsafe { is_in_main_thread() });

    let stops_slice = unsafe { slice::from_raw_parts(stops, stops_count) };
    let stops_vector = stops_slice.iter().map(|x| x.into()).collect();

    let gradient = state.frame_builder
                        .dl_builder
                        .create_radial_gradient(center.into(),
                                                radius.into(),
                                                stops_vector,
                                                extend_mode.into());
    let rect = rect.into();
    let tile_size = tile_size.into();
    let tile_spacing = tile_spacing.into();
    state.frame_builder
         .dl_builder
         .push_radial_gradient(rect, clip.into(), gradient, tile_size, tile_spacing);
}

#[no_mangle]
pub extern "C" fn wr_dp_push_box_shadow(state: &mut WrState,
                                        rect: WrRect,
                                        clip: WrClipRegion,
                                        box_bounds: WrRect,
                                        offset: WrPoint,
                                        color: WrColor,
                                        blur_radius: f32,
                                        spread_radius: f32,
                                        border_radius: f32,
                                        clip_mode: WrBoxShadowClipMode) {
    assert!(unsafe { is_in_main_thread() });

    state.frame_builder
         .dl_builder
         .push_box_shadow(rect.into(),
                          clip.into(),
                          box_bounds.into(),
                          offset.into(),
                          color.into(),
                          blur_radius,
                          spread_radius,
                          border_radius,
                          clip_mode);
}

#[no_mangle]
pub unsafe extern "C" fn wr_api_finalize_builder(state: &mut WrState,
                                                 dl_descriptor: &mut WrBuiltDisplayListDescriptor,
                                                 dl_data: &mut WrVecU8,
                                                 aux_descriptor: &mut WrAuxiliaryListsDescriptor,
                                                 aux_data: &mut WrVecU8) {
    let frame_builder = mem::replace(&mut state.frame_builder,
                                     WebRenderFrameBuilder::new(state.pipeline_id));
    let (_, dl, aux) = frame_builder.dl_builder.finalize();
    let (data, descriptor) = dl.into_data();
    *dl_data = WrVecU8::from_vec(data);
    *dl_descriptor = descriptor;

    let (data, descriptor) = aux.into_data();
    *aux_data = WrVecU8::from_vec(data);
    *aux_descriptor = descriptor;
}

#[no_mangle]
pub unsafe extern "C" fn wr_dp_push_built_display_list(state: &mut WrState,
                                                       dl_descriptor: WrBuiltDisplayListDescriptor,
                                                       dl_data: WrVecU8,
                                                       aux_descriptor: WrAuxiliaryListsDescriptor,
                                                       aux_data: WrVecU8) {
    let dl_vec = dl_data.to_vec();
    let aux_vec = aux_data.to_vec();

    let dl = BuiltDisplayList::from_data(dl_vec, dl_descriptor);
    let aux = AuxiliaryLists::from_data(aux_vec, aux_descriptor);

    state.frame_builder.dl_builder.push_built_display_list(dl, aux);
}

struct Moz2dImageRenderer {
    images: HashMap<WrImageKey, BlobImageResult>,
}

impl BlobImageRenderer for Moz2dImageRenderer {
    fn request_blob_image(&mut self,
                          key: WrImageKey,
                          data: Arc<BlobImageData>,
                          descriptor: &BlobImageDescriptor,
                          _dirty_rect: Option<DeviceUintRect>) {
        let result = self.render_blob_image(data, descriptor);
        self.images.insert(key, result);
    }

    fn resolve_blob_image(&mut self,
                          key: WrImageKey)
                          -> BlobImageResult {
        return match self.images.remove(&key) {
                   Some(result) => result,
                   None => Err(BlobImageError::InvalidKey),
               };
    }
}

impl Moz2dImageRenderer {
    fn new() -> Self {
        Moz2dImageRenderer {
            images: HashMap::new(),
        }
    }

    fn render_blob_image(&mut self,
                         data: Arc<BlobImageData>,
                         descriptor: &BlobImageDescriptor)
                         -> BlobImageResult {
        let mut output = Vec::with_capacity((descriptor.width * descriptor.height *
                                             descriptor.format.bytes_per_pixel().unwrap()) as
                                            usize);

        unsafe {
            if wr_moz2d_render_cb(WrByteSlice::new(&data[..]),
                                  descriptor.width,
                                  descriptor.height,
                                  descriptor.format,
                                  MutByteSlice::new(output.as_mut_slice())) {
                return Ok(RasterizedBlobImage {
                              width: descriptor.width,
                              height: descriptor.height,
                              data: output,
                          });
            }
        }

        Err(BlobImageError::Other("unimplemented!".to_string()))
    }
}

extern "C" {
    // TODO: figure out the API for tiled blob images.
    fn wr_moz2d_render_cb(blob: WrByteSlice,
                          width: u32,
                          height: u32,
                          format: WrImageFormat,
                          output: MutByteSlice)
                          -> bool;
}
