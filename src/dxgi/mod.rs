use self::ffi::*;
use std::{io, mem, ptr, slice};
use winapi::um::d3d11::D3D11_USAGE_STAGING;
use winapi::um::d3d11::D3D11_CPU_ACCESS_READ;
use winapi::shared::dxgi::DXGI_RESOURCE_PRIORITY_MAXIMUM;
use winapi::shared::dxgi::IDXGIResource;
use winapi::shared::dxgi::IDXGISurface;
use winapi::shared::minwindef::TRUE;
use winapi::shared::winerror::DXGI_ERROR_SESSION_DISCONNECTED;
use winapi::shared::winerror::DXGI_ERROR_NOT_CURRENTLY_AVAILABLE;
use winapi::um::d3d11::ID3D11Texture2D;
use winapi::shared::winerror::DXGI_ERROR_UNSUPPORTED;
use winapi::shared::winerror::E_ACCESSDENIED;
use winapi::shared::winerror::DXGI_ERROR_INVALID_CALL;
use winapi::shared::winerror::DXGI_ERROR_WAIT_TIMEOUT;
use winapi::shared::winerror::DXGI_ERROR_ACCESS_LOST;
use winapi::um::d3dcommon::D3D_FEATURE_LEVEL_9_1;
use winapi::um::d3d11::D3D11_SDK_VERSION;
use winapi::um::d3dcommon::D3D_DRIVER_TYPE_UNKNOWN;
use winapi::shared::dxgi1_2::IDXGIOutputDuplication;
use winapi::shared::dxgitype::DXGI_MODE_ROTATION;
use winapi::shared::ntdef::LONG;
use winapi::shared::dxgi::DXGI_OUTPUT_DESC;
use winapi::shared::minwindef::UINT;
use winapi::shared::winerror::S_OK;
use winapi::shared::dxgi1_2::IDXGIOutput1;
use winapi::shared::dxgi::IDXGIFactory1;
use winapi::shared::dxgi::IDXGIAdapter1;
use winapi::shared::winerror::HRESULT;
use winapi::um::d3d11::ID3D11Device;
use winapi::um::d3d11::ID3D11DeviceContext;
use winapi::um::unknwnbase::IUnknown;
use winapi::um::d3d11::ID3D11Resource;

mod ffi;

//TODO: Split up into files.

pub struct Capturer {
    device: *mut ID3D11Device,
    context: *mut ID3D11DeviceContext,
    duplication: *mut IDXGIOutputDuplication,
    fastlane: bool, surface: *mut IDXGISurface,
    data: *mut u8, len: usize,
    height: usize,
}

impl Capturer {
    pub fn new(display: &Display) -> io::Result<Capturer> {
        let mut device = ptr::null_mut();
        let mut context = ptr::null_mut();
        let mut duplication = ptr::null_mut();
        let mut desc = mem::MaybeUninit::uninit();

        if unsafe {
            D3D11CreateDevice(
                display.adapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                ptr::null_mut(), // No software rasterizer.
                0, // No device flags.
                ptr::null_mut(), // Feature levels.
                0, // Feature levels' length.
                D3D11_SDK_VERSION,
                &mut device,
                #[allow(const_item_mutation)]
                &mut D3D_FEATURE_LEVEL_9_1,
                &mut context
            )
        } != S_OK {
            // Unknown error.
            return Err(io::ErrorKind::Other.into());
        }
       
        let res = wrap_hresult(unsafe {
            (*display.inner).DuplicateOutput(device  as *mut IUnknown,
                &mut duplication
            )
        });

        if let Err(err) = res {
            unsafe {
                (*device).Release();
                (*context).Release();
            }
            return Err(err);
        }
        
        unsafe {
            (*duplication).GetDesc(desc.assume_init_mut());
        }

        Ok(unsafe {
            let mut capturer = Capturer {
                device, context, duplication,
                fastlane: desc.assume_init_mut().DesktopImageInSystemMemory == TRUE,
                surface: ptr::null_mut(),
                height: display.height() as usize,
                data: ptr::null_mut(),
                len: 0
            };
            let _ = capturer.load_frame(0);
            capturer
        })
    }

    unsafe fn load_frame(&mut self, timeout: UINT) -> io::Result<()> {
        let mut frame = ptr::null_mut();
        let mut info = mem::MaybeUninit::uninit();
        self.data = ptr::null_mut();

        wrap_hresult((*self.duplication).AcquireNextFrame(
            timeout,
            info.assume_init_mut(),
            &mut frame
        ))?;

        if self.fastlane {
            let mut rect = mem::MaybeUninit::uninit();
            let res = wrap_hresult(
                (*self.duplication).MapDesktopSurface( rect.assume_init_mut())
            );

            (*frame).Release();

            if let Err(err) = res {
                Err(err)
            } else {
                self.data = rect.assume_init_ref().pBits;
                self.len = self.height * rect.assume_init_ref().Pitch as usize;
                Ok(())
            }
        } else {
            self.surface = ptr::null_mut();
            self.surface = self.ohgodwhat(frame)?;

            let mut rect = mem::MaybeUninit::uninit();
            wrap_hresult((*self.surface).Map(
                rect.assume_init_mut(),
                DXGI_MAP_READ
            ))?;

            self.data = rect.assume_init_ref().pBits;
            self.len = self.height * rect.assume_init_ref().Pitch as usize;
            Ok(())
        }
    }

    unsafe fn ohgodwhat(
        &mut self,
        frame: *mut IDXGIResource
    ) -> io::Result<*mut IDXGISurface> {
        let mut texture: *mut ID3D11Texture2D = ptr::null_mut();
        (*frame).QueryInterface(
            &IID_ID3D11TEXTURE2D,
            &mut texture as *mut *mut _ as *mut *mut _
        );

        let mut texture_desc = mem::MaybeUninit::uninit();
        (*texture).GetDesc(texture_desc.assume_init_mut());

        texture_desc.assume_init_mut().Usage = D3D11_USAGE_STAGING;
        texture_desc.assume_init_mut().BindFlags = 0;
        texture_desc.assume_init_mut().CPUAccessFlags = D3D11_CPU_ACCESS_READ;
        texture_desc.assume_init_mut().MiscFlags = 0;

        let mut readable = ptr::null_mut();
        let res = wrap_hresult((*self.device).CreateTexture2D(
            texture_desc.assume_init_mut(),
            ptr::null(),
            &mut readable
        ));

        if let Err(err) = res {
            (*frame).Release();
            (*texture).Release();
            (*readable).Release();
            Err(err)
        } else {
            (*readable).SetEvictionPriority(DXGI_RESOURCE_PRIORITY_MAXIMUM);

            let mut surface = ptr::null_mut();
            (*readable).QueryInterface(
                &IID_IDXGISURFACE,
                &mut surface as *mut *mut _ as *mut *mut _
            );

            (*self.context).CopyResource(
                readable as *mut ID3D11Resource,
                texture as *mut ID3D11Resource,
            );

            (*frame).Release();
            (*texture).Release();
            (*readable).Release();
            Ok(surface)
        }
    }

    pub fn frame<'a>(&'a mut self, timeout: UINT) -> io::Result<&'a [u8]> {
        unsafe {
            // Release last frame.
            // No error checking needed because we don't care.
            // None of the errors crash anyway.

            if self.fastlane {
                (*self.duplication).UnMapDesktopSurface();
            } else {
                if !self.surface.is_null() {
                    (*self.surface).Unmap();
                    (*self.surface).Release();
                    self.surface = ptr::null_mut();
                }
            }

            (*self.duplication).ReleaseFrame();

            // Get next frame.

            self.load_frame(timeout)?;
            Ok(slice::from_raw_parts(self.data, self.len))
        }
    }
}

impl Drop for Capturer {
    fn drop(&mut self) {
        unsafe {
            if !self.surface.is_null() {
                (*self.surface).Unmap();
                (*self.surface).Release();
            }
            (*self.duplication).Release();
            (*self.device).Release();
            (*self.context).Release();
        }
    }
}

pub struct Displays {
    factory: *mut IDXGIFactory1,
    adapter: *mut IDXGIAdapter1,
    /// Index of the CURRENT adapter.
    nadapter: UINT,
    /// Index of the NEXT display to fetch.
    ndisplay: UINT
}

impl Displays {
    pub fn new() -> io::Result<Displays> {
        let mut factory = ptr::null_mut();
        wrap_hresult(unsafe {
            CreateDXGIFactory1(&IID_IDXGIFACTORY1, &mut factory)
        })?;

        let mut adapter = ptr::null_mut();
        unsafe {
            // On error, our adapter is null, so it's fine.
            (*factory).EnumAdapters1(0, &mut adapter);
        };

        Ok(Displays {
            factory,
            adapter,
            nadapter: 0,
            ndisplay: 0
        })
    }

    // No Adapter => Some(None)
    // Non-Empty Adapter => Some(Some(OUTPUT))
    // End of Adapter => None
    fn read_and_invalidate(&mut self) -> Option<Option<Display>> {
        // If there is no adapter, there is nothing left for us to do.

        if self.adapter.is_null() {
            return Some(None);
        }

        // Otherwise, we get the next output of the current adapter.

        let output = unsafe {
            let mut output = ptr::null_mut();
            (*self.adapter).EnumOutputs(self.ndisplay, &mut output);
            output
        };

        // If the current adapter is done, we free it.
        // We return None so the caller gets the next adapter and tries again.

        if output.is_null() {
            unsafe {
                (*self.adapter).Release();
                self.adapter = ptr::null_mut();
            }
            return None;
        }

        // Advance to the next display.

        self.ndisplay += 1;

        // We get the display's details.

        let desc = unsafe {
            let mut desc = mem::MaybeUninit::uninit();
            (*output).GetDesc(desc.assume_init_mut());
            desc
        };

        // We cast it up to the version needed for desktop duplication.

        let mut inner = ptr::null_mut();
        unsafe {
            (*output).QueryInterface(
                &IID_IDXGIOUTPUT1,
                &mut inner 
            );
            (*output).Release();
        }

        // If it's null, we have an error.
        // So we act like the adapter is done.

        if inner.is_null() {
            unsafe {
                (*self.adapter).Release();
                self.adapter = ptr::null_mut();
            }
            return None;
        }

        unsafe {
            (*self.adapter).AddRef();
        }
        
        Some(Some(Display { inner:inner as *mut IDXGIOutput1, adapter: self.adapter, desc:unsafe{desc.assume_init()} }))
    }
}

impl Iterator for Displays {
    type Item = Display;
    fn next(&mut self) -> Option<Display> {
        if let Some(res) = self.read_and_invalidate() {
            res
        } else {
            // We need to replace the adapter.

            self.ndisplay = 0;
            self.nadapter += 1;

            self.adapter = unsafe {
                let mut adapter = ptr::null_mut();
                (*self.factory).EnumAdapters1(
                    self.nadapter,
                    &mut adapter
                );
                adapter
            };

            if let Some(res) = self.read_and_invalidate() {
                res
            } else {
                // All subsequent adapters will also be empty.
                None
            }
        }
    }
}

impl Drop for Displays {
    fn drop(&mut self) {
        unsafe {
            (*self.factory).Release();
            if !self.adapter.is_null() {
                (*self.adapter).Release();
            }
        }
    }
}

pub struct Display {
    inner: *mut IDXGIOutput1,
    adapter: *mut IDXGIAdapter1,
    desc: DXGI_OUTPUT_DESC
}

impl Display {
    pub fn width(&self) -> LONG {
        self.desc.DesktopCoordinates.right -
        self.desc.DesktopCoordinates.left
    }

    pub fn height(&self) -> LONG {
        self.desc.DesktopCoordinates.bottom -
        self.desc.DesktopCoordinates.top
    }

    pub fn rotation(&self) -> DXGI_MODE_ROTATION {
        self.desc.Rotation
    }

    pub fn name(&self) -> &[u16] {
        let s = &self.desc.DeviceName;
        let i = s.iter()
            .position(|&x| x == 0)
            .unwrap_or(s.len());
        &s[..i]
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            (*self.inner).Release();
            (*self.adapter).Release();
        }
    }
}

fn wrap_hresult(x: HRESULT) -> io::Result<()> {
    use std::io::ErrorKind::*;
    Err((match x {
        S_OK => return Ok(()),
        DXGI_ERROR_ACCESS_LOST => ConnectionReset,
        DXGI_ERROR_WAIT_TIMEOUT => TimedOut,
        DXGI_ERROR_INVALID_CALL => InvalidData,
        E_ACCESSDENIED => PermissionDenied,
        DXGI_ERROR_UNSUPPORTED => ConnectionRefused,
        DXGI_ERROR_NOT_CURRENTLY_AVAILABLE => Interrupted,
        DXGI_ERROR_SESSION_DISCONNECTED => ConnectionAborted,
        _ => Other
    }).into())
}