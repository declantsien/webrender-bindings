/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifndef WR_h
#define WR_h
extern "C" {
bool is_in_compositor_thread();

enum WRImageFormat {
    Invalid,
    A8,
    RGB8,
    RGBA8,
    RGBAF32
};

struct WRImageKey {
  uint32_t a;
  uint32_t b;

  bool operator==(const WRImageKey& aRhs) const {
    return a == aRhs.a && b == aRhs.b;
  }
};

struct WRRect {
  float x;
  float y;
  float width;
  float height;

  bool operator==(const WRRect& aRhs) const {
    return x == aRhs.x && y == aRhs.y &&
           width == aRhs.width && height == aRhs.height;
  }
};

struct WRImageMask
{
    WRImageKey image;
    WRRect rect;
    bool repeat;

    bool operator==(const WRImageMask& aRhs) const {
      return image == aRhs.image && rect == aRhs.rect && repeat == aRhs.repeat;
    }
};

typedef uint64_t WRImageIdType;
struct WRExternalImageId {
  WRImageIdType id;
};

enum WRExternalImageType {
    TEXTURE_HANDLE, // Currently, we only support gl texture handle.
    // TODO(Jerry): handle shmem or cpu raw buffers.
    //// MEM_OR_SHMEM,
};

struct WRExternalImage {
  WRExternalImageType type;

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

typedef WRExternalImage (*GetExternalImageCallback)(void*, WRExternalImageId);
typedef void (*ReleaseExternalImageCallback)(void*, WRExternalImageId);

struct WRExternalImageHandler {
  void* ExternalImageObj;
  GetExternalImageCallback get_func;
  ReleaseExternalImageCallback release_func;
};

struct wrwindowstate;
struct wrstate;

#ifdef MOZ_ENABLE_WEBRENDER
#  define WR_INLINE
#  define WR_FUNC
#else
#  define WR_INLINE inline
#  define WR_FUNC { MOZ_MAKE_COMPILER_ASSUME_IS_UNREACHABLE("WebRender disabled"); }
#endif

WR_INLINE wrwindowstate*
wr_init_window(uint64_t root_pipeline_id,
               bool enable_profiler,
               WRExternalImageHandler* handler = nullptr)
WR_FUNC;

WR_INLINE wrstate*
wr_create(wrwindowstate* wrWindow, uint32_t width, uint32_t height, uint64_t layers_id)
WR_FUNC;

WR_INLINE void
wr_destroy(wrstate* wrstate)
WR_FUNC;

WR_INLINE WRImageKey
wr_add_image(wrwindowstate* wrWindow, uint32_t width, uint32_t height,
             uint32_t stride, WRImageFormat format, uint8_t *bytes, size_t size)
WR_FUNC;

WR_INLINE WRImageKey
wr_add_external_image_texture(wrwindowstate* wrWindow, uint32_t width, uint32_t height,
                              WRImageFormat format, uint64_t external_image_id)
WR_FUNC;

//TODO(Jerry): handle shmem in WR
//// WR_INLINE WRImageKey
//// wr_add_external_image_buffer(wrwindowstate* wrWindow, uint32_t width, uint32_t height,
////                              uint32_t stride, WRImageFormat format, uint8_t *bytes, size_t size)
//// WR_FUNC;

WR_INLINE void
wr_update_image(wrwindowstate* wrWindow, WRImageKey key,
                uint32_t width, uint32_t height,
                WRImageFormat format, uint8_t *bytes, size_t size)
WR_FUNC;

WR_INLINE void
wr_delete_image(wrwindowstate* wrWindow, WRImageKey key)
WR_FUNC;

WR_INLINE void
wr_push_dl_builder(wrstate *wrState)
WR_FUNC;

//XXX: matrix should use a proper type
WR_INLINE void
wr_pop_dl_builder(wrstate *wrState, WRRect bounds,
                  WRRect overflow, const float* matrix)
WR_FUNC;

WR_INLINE void
wr_dp_begin(wrwindowstate* wrWindow, wrstate* wrState, uint32_t width, uint32_t height)
WR_FUNC;

WR_INLINE void
wr_dp_end(wrwindowstate* wrWindow, wrstate* wrState, uint32_t epoch)
WR_FUNC;

WR_INLINE void
wr_composite_window(wrwindowstate* wrWindow)
WR_FUNC;

WR_INLINE void
wr_dp_push_rect(wrstate* wrState, WRRect bounds, WRRect clip,
                float r, float g, float b, float a)
WR_FUNC;

WR_INLINE void
wr_dp_push_image(wrstate* wrState, WRRect bounds, WRRect clip,
                 const WRImageMask* mask, WRImageKey key)
WR_FUNC;

WR_INLINE void
wr_dp_push_iframe(wrstate* wrState, WRRect bounds, WRRect clip,
                  uint64_t layers_id)
WR_FUNC;

// The pointer returned by wr_readback_buffer must be freed by rust, not C.
// After using the data, it is the responsibility of the caller to free the memory
// by giving the pointer, out_length, out_capacity to wr_free_buffer.
WR_INLINE const uint8_t*
wr_readback_buffer(wrwindowstate* wrWindow, uint32_t width, uint32_t height,
                   uint32_t* out_length, uint32_t* out_capacity)
WR_FUNC;

WR_INLINE void
wr_free_buffer(const uint8_t* pointer, uint32_t length, uint32_t capacity)

WR_FUNC;

WR_INLINE void
wr_profiler_set_enabled(wrwindowstate* wrWindow, bool enabled)
WR_FUNC;

#undef WR_FUNC
}
#endif
