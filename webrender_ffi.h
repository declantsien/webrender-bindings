/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifndef WR_h
#define WR_h

#include "mozilla/layers/LayersMessages.h"
#include "mozilla/gfx/Types.h"

extern "C" {
bool is_in_compositor_thread();
bool is_in_render_thread();
void* get_proc_address_from_glcontext(void* glcontext_ptr, const char* procname);

enum WrImageFormat {
    Invalid,
    A8,
    RGB8,
    RGBA8,
    RGBAF32
};

typedef uint64_t WrWindowId;
typedef uint64_t WrImageKey;
typedef uint64_t WrFontKey;
typedef uint64_t WrPipelineId;
typedef uint32_t WrEpoch;

struct WrColor {
  float r;
  float g;
  float b;
  float a;

  bool operator==(const WrColor& aRhs) const {
    return r == aRhs.r && g == aRhs.g &&
           b == aRhs.b && a == aRhs.a;
  }
};

struct WRGlyphInstance {
  uint32_t index;
  float x;
  float y;

  bool operator==(const WRGlyphInstance& other) const {
    return index == other.index &&
           x == other.x &&
           y == other.y;
  }
};

// Note that the type is slightly different than
// the definition in bindings.rs. WRGlyphInstance
// versus GlyphInstance, but their layout is the same.
// So we're really overlapping the types for the same memory.
struct WrGlyphArray {
  mozilla::gfx::Color color;
  nsTArray<WRGlyphInstance> glyphs;

  bool operator==(const WrGlyphArray& other) const {
    if (!(color == other.color) ||
       (glyphs.Length() != other.glyphs.Length())) {
      return false;
    }

    for (size_t i = 0; i < glyphs.Length(); i++) {
      if (!(glyphs[i] == other.glyphs[i])) {
        return false;
      }
    }

    return true;
  }
};

enum WrBorderStyle {
  None,
  Solid,
  Double,
  Dotted,
  Dashed,
  Hidden,
  Groove,
  Ridge,
  Inset,
  Outset
};

struct WrBorderSide {
  float width;
  WrColor color;
  WrBorderStyle style;

  bool operator==(const WrBorderSide& aRhs) const {
    return width == aRhs.width && color == aRhs.color &&
           style == aRhs.style;
  }
};

struct WrLayoutSize {
  float width;
  float height;

  bool operator==(const WrLayoutSize& aRhs) const {
    return width == aRhs.width && height == aRhs.height;
  }
};

struct WrRect {
  float x;
  float y;
  float width;
  float height;

  bool operator==(const WrRect& aRhs) const {
    return x == aRhs.x && y == aRhs.y &&
           width == aRhs.width && height == aRhs.height;
  }
};

struct WrImageMask
{
    WrImageKey image;
    WrRect rect;
    bool repeat;

    bool operator==(const WrImageMask& aRhs) const {
      return image == aRhs.image && rect == aRhs.rect && repeat == aRhs.repeat;
    }
};

enum class WrTextureFilter
{
  Linear,
  Point,
  Sentinel,
};

typedef uint64_t WrImageIdType;
struct WrExternalImageIdId {
  WrImageIdType id;
};

enum WrExternalImageIdType {
    TEXTURE_HANDLE, // Currently, we only support gl texture handle.
    // TODO(Jerry): handle shmem or cpu raw buffers.
    //// MEM_OR_SHMEM,
};

struct WrExternalImageId {
  WrExternalImageIdType type;

  // Texture coordinate
  float u0, v0;
  float u1, v1;

  // external buffer handle
  uint32_t handle;

  // TODO(Jerry): handle shmem or cpu raw buffers.
  //// shmem or memory buffer
  //// uint8_t* buff;
  //// size_t size;
};

typedef WrExternalImageId (*LockExternalImageCallback)(void*, WrExternalImageIdId);
typedef void (*UnlockExternalImageCallback)(void*, WrExternalImageIdId);
typedef void (*ReleaseExternalImageCallback)(void*, WrExternalImageIdId);

struct WrExternalImageIdHandler {
  void* ExternalImageObj;
  LockExternalImageCallback lock_func;
  UnlockExternalImageCallback unlock_func;
  ReleaseExternalImageCallback release_func;
};

struct WrWindowState;

#ifdef MOZ_ENABLE_WEBRENDER
#  define WR_INLINE
#  define WR_FUNC
#else
#  define WR_INLINE inline
#  define WR_FUNC { MOZ_MAKE_COMPILER_ASSUME_IS_UNREACHABLE("WebRender disabled"); }
#endif

struct WrRenderer;
struct WrState;

WR_INLINE void
wr_renderer_update(WrRenderer* renderer) WR_FUNC;

WR_INLINE void
wr_renderer_render(WrRenderer* renderer, uint32_t width, uint32_t height) WR_FUNC;

WR_INLINE void
wr_renderer_set_profiler_enabled(WrRenderer* renderer, bool enabled) WR_FUNC;

WR_INLINE bool
wr_renderer_current_epoch(WrRenderer* renderer, WrPipelineId pipeline_id, WrEpoch* out_epoch) WR_FUNC;

WR_INLINE bool
wr_renderer_delete(WrRenderer* renderer) WR_FUNC;

WR_INLINE void
wr_gl_init(void* aGLContext) WR_FUNC;

struct WrAPI;

WR_INLINE void
wr_window_new(WrWindowId window_id,
              bool enable_profiler,
              WrAPI** out_api,
              WrRenderer** out_renderer) WR_FUNC;

WR_INLINE void
wr_window_remove_pipeline(WrWindowState* window, WrState* state) WR_FUNC;

WR_INLINE void
wr_api_delete(WrAPI* api) WR_FUNC;

WR_INLINE WrImageKey
wr_api_add_image(WrAPI* api, uint32_t width, uint32_t height,
                 uint32_t stride, WrImageFormat format, uint8_t *bytes, size_t size) WR_FUNC;

WR_INLINE WrImageKey
wr_api_add_external_image_texture(WrAPI* api, uint32_t width, uint32_t height,
                                  WrImageFormat format, uint64_t external_image_id) WR_FUNC;

WR_INLINE void
wr_api_update_image(WrAPI* api, WrImageKey key,
                    uint32_t width, uint32_t height,
                    WrImageFormat format, uint8_t *bytes, size_t size) WR_FUNC;

WR_INLINE void
wr_api_delete_image(WrAPI* api, WrImageKey key) WR_FUNC;

WR_INLINE void
wr_api_set_root_pipeline(WrAPI* api, WrPipelineId pipeline_id) WR_FUNC;

WR_INLINE void
wr_api_set_root_display_list(WrAPI* api, WrState* state, uint32_t epoch, float w, float h) WR_FUNC;

WR_INLINE void
wr_api_send_external_event(WrAPI* api, uintptr_t evt) WR_FUNC;

WR_INLINE void
wr_window_init_pipeline_epoch(WrWindowState* window, WrPipelineId pipeline, uint32_t width, uint32_t height) WR_FUNC;

WR_INLINE WrFontKey
wr_api_add_raw_font(WrAPI* api, uint8_t* font_buffer, size_t buffer_size) WR_FUNC;

WR_INLINE WrFontKey
wr_window_add_raw_font(WrWindowState* window, uint8_t* font_buffer, size_t buffer_size) WR_FUNC;

WR_INLINE WrWindowState*
wr_init_window(WrPipelineId root_pipeline_id,
               void* webrender_bridge_ptr,
               bool enable_profiler,
               WrExternalImageIdHandler* handler = nullptr)
WR_FUNC;

WR_INLINE WrState*
wr_state_new(uint32_t width, uint32_t height, WrPipelineId pipeline_id) WR_FUNC;

WR_INLINE void
wr_state_delete(WrState* state) WR_FUNC;

WR_INLINE void
wr_destroy(WrWindowState* wrWindow, WrState* WrState)
WR_FUNC;

WR_INLINE WrImageKey
wr_add_image(WrWindowState* wrWindow, uint32_t width, uint32_t height,
             uint32_t stride, WrImageFormat format, uint8_t *bytes, size_t size)
WR_FUNC;

WR_INLINE WrImageKey
wr_add_external_image_texture(WrWindowState* wrWindow, uint32_t width, uint32_t height,
                              WrImageFormat format, uint64_t external_image_id)
WR_FUNC;

//TODO(Jerry): handle shmem in WR
//// WR_INLINE WrImageKey
//// wr_add_external_image_buffer(WrWindowState* wrWindow, uint32_t width, uint32_t height,
////                              uint32_t stride, WrImageFormat format, uint8_t *bytes, size_t size)
//// WR_FUNC;

WR_INLINE void
wr_update_image(WrWindowState* wrWindow, WrImageKey key,
                uint32_t width, uint32_t height,
                WrImageFormat format, uint8_t *bytes, size_t size)
WR_FUNC;

WR_INLINE void
wr_delete_image(WrWindowState* wrWindow, WrImageKey key)
WR_FUNC;

WR_INLINE void
wr_dp_push_stacking_context(WrState *wrState, WrRect bounds,
                            WrRect overflow, const WrImageMask *mask,
                            const float* matrix)
WR_FUNC;

//XXX: matrix should use a proper type
WR_INLINE void
wr_dp_pop_stacking_context(WrState *wrState)
WR_FUNC;

WR_INLINE void
wr_dp_begin(WrState* wrState, uint32_t width, uint32_t height)
WR_FUNC;

WR_INLINE void
wr_window_dp_begin(WrWindowState* wrWindow, WrState* wrState, uint32_t width, uint32_t height)
WR_FUNC;

WR_INLINE void
wr_window_dp_end(WrWindowState* wrWindow, WrState* wrState)
WR_FUNC;

WR_INLINE void
wr_dp_end(WrState* builder, WrAPI* api, uint32_t epoch) WR_FUNC;

WR_INLINE void
wr_composite_window(WrWindowState* wrWindow)
WR_FUNC;

WR_INLINE void
wr_dp_push_rect(WrState* wrState, WrRect bounds, WrRect clip,
                float r, float g, float b, float a)
WR_FUNC;

WR_INLINE void
wr_dp_push_text(WrState* wrState,
                WrRect bounds, WrRect clip,
                WrColor color,
                WrFontKey font_Key,
                const WRGlyphInstance* glyphs,
                uint32_t glyph_count, float glyph_size) WR_FUNC;

WR_INLINE void
wr_dp_push_border(WrState* wrState, WrRect bounds, WrRect clip,
                  WrBorderSide top, WrBorderSide right, WrBorderSide bottom, WrBorderSide left,
                  WrLayoutSize top_left_radius, WrLayoutSize top_right_radius,
                  WrLayoutSize bottom_left_radius, WrLayoutSize bottom_right_radius)
WR_FUNC;

WR_INLINE void
wr_dp_push_image(WrState* wrState, WrRect bounds, WrRect clip,
                 const WrImageMask* mask, WrTextureFilter filter, WrImageKey key)
WR_FUNC;

// TODO: Remove.
WR_INLINE void
wr_window_dp_push_iframe(WrWindowState* wrWindow, WrState* wrState, WrRect bounds, WrRect clip,
                        WrPipelineId layers_id)
WR_FUNC;

WR_INLINE void
wr_dp_push_iframe(WrState* wrState, WrRect bounds, WrRect clip, WrPipelineId layers_id) WR_FUNC;

// TODO: Remove.
// It is the responsibility of the caller to manage the dst_buffer memory
// and also free it at the proper time.
WR_INLINE const uint8_t*
wr_readback_into_buffer(WrWindowState* wrWindow, uint32_t width, uint32_t height,
                        uint8_t* dst_buffer, uint32_t buffer_length)
WR_FUNC;

// TODO: Remove.
WR_INLINE void
wr_profiler_set_enabled(WrWindowState* wrWindow, bool enabled)
WR_FUNC;

#undef WR_FUNC
}
#endif
