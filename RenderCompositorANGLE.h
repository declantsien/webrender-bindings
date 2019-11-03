/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifndef MOZILLA_GFX_RENDERCOMPOSITOR_ANGLE_H
#define MOZILLA_GFX_RENDERCOMPOSITOR_ANGLE_H

#include <queue>

#include "GLTypes.h"
#include "mozilla/Maybe.h"
#include "mozilla/webrender/RenderCompositor.h"
#include "mozilla/webrender/RenderThread.h"

struct ID3D11DeviceContext;
struct ID3D11Device;
struct ID3D11Query;
struct IDXGIFactory2;
struct IDXGISwapChain;
struct IDXGISwapChain1;

namespace mozilla {
namespace gl {
class GLLibraryEGL;
}  // namespace gl

namespace wr {

class DCLayerTree;

class RenderCompositorANGLE : public RenderCompositor {
 public:
  static UniquePtr<RenderCompositor> Create(
      RefPtr<widget::CompositorWidget>&& aWidget);

  explicit RenderCompositorANGLE(RefPtr<widget::CompositorWidget>&& aWidget);
  virtual ~RenderCompositorANGLE();
  bool Initialize();

  bool BeginFrame() override;
  void EndFrame(const FfiVec<DeviceIntRect>& aDirtyRects) override;
  bool WaitForGPU() override;
  void Pause() override;
  bool Resume() override;
  void Update() override;

  gl::GLContext* gl() const override { return RenderThread::Get()->SharedGL(); }

  bool MakeCurrent() override;

  bool UseANGLE() const override { return true; }

  bool UseDComp() const override { return !!mDCLayerTree; }

  bool UseTripleBuffering() const override { return mUseTripleBuffering; }

  LayoutDeviceIntSize GetBufferSize() override;

  bool IsContextLost() override;

  bool SurfaceIsYFlipped() override { return true; }

  bool ShouldUseNativeCompositor() override;

  // Interface for wr::Compositor
  void CompositorBeginFrame() override;
  void CompositorEndFrame() override;
  void Bind(wr::NativeSurfaceId aId, wr::DeviceIntPoint* aOffset,
            uint32_t* aFboId, wr::DeviceIntRect aDirtyRect) override;
  void Unbind() override;
  void CreateSurface(wr::NativeSurfaceId aId, wr::DeviceIntSize aSize,
                     bool aIsOpaque) override;
  void DestroySurface(NativeSurfaceId aId) override;
  void AddSurface(wr::NativeSurfaceId aId, wr::DeviceIntPoint aPosition,
                  wr::DeviceIntRect aClipRect) override;

  // Interface for partial present
  bool RequestFullRender() override;
  uint32_t GetMaxPartialPresentRects() override;

 protected:
  bool UseCompositor();
  void InitializeUsePartialPresent();
  void InsertPresentWaitQuery();
  bool WaitForPreviousPresentQuery();
  bool ResizeBufferIfNeeded();
  bool CreateEGLSurface();
  void DestroyEGLSurface();
  ID3D11Device* GetDeviceOfEGLDisplay();
  void CreateSwapChainForDCompIfPossible(IDXGIFactory2* aDXGIFactory2);
  RefPtr<IDXGISwapChain1> CreateSwapChainForDComp(bool aUseTripleBuffering,
                                                  bool aUseAlpha);
  bool SutdownEGLLibraryIfNecessary();
  RefPtr<ID3D11Query> GetD3D11Query();

  EGLConfig mEGLConfig;
  EGLSurface mEGLSurface;

  bool mUseTripleBuffering;
  bool mUseAlpha;

  RefPtr<ID3D11Device> mDevice;
  RefPtr<ID3D11DeviceContext> mCtx;
  RefPtr<IDXGISwapChain> mSwapChain;
  RefPtr<IDXGISwapChain1> mSwapChain1;

  UniquePtr<DCLayerTree> mDCLayerTree;

  std::queue<RefPtr<ID3D11Query>> mWaitForPresentQueries;
  RefPtr<ID3D11Query> mRecycledQuery;

  Maybe<LayoutDeviceIntSize> mBufferSize;
  bool mUsePartialPresent;
  bool mFullRender;
};

}  // namespace wr
}  // namespace mozilla

#endif
