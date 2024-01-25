use winapi::shared::guiddef::GUID;
use winapi::shared::winerror::HRESULT;
use winapi::shared::guiddef::REFIID;
use winapi::shared::dxgi::IDXGIFactory1;
use winapi::shared::dxgi::IDXGIAdapter1;
use winapi::um::d3dcommon::D3D_DRIVER_TYPE;
use winapi::shared::minwindef::HMODULE;
use winapi::shared::minwindef::UINT;
use winapi::um::d3d11::ID3D11Device;
use winapi::um::d3dcommon::D3D_FEATURE_LEVEL;
use winapi::um::d3d11::ID3D11DeviceContext;

pub const DXGI_MAP_READ: UINT = 1;

pub const IID_IDXGIFACTORY1: GUID = GUID {
    Data1: 0x770aae78,
    Data2: 0xf26f,
    Data3: 0x4dba,
    Data4: [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87]
};

pub const IID_IDXGIOUTPUT1: GUID = GUID {
    Data1: 0x00cddea8,
    Data2: 0x939b,
    Data3: 0x4b83,
    Data4: [0xa3, 0x40, 0xa6, 0x85, 0x22, 0x66, 0x66, 0xcc]
};

pub const IID_IDXGISURFACE: GUID = GUID {
    Data1: 3405559148,
    Data2: 27331,
    Data3: 18569,
    Data4: [191, 71, 158, 35, 187, 210, 96, 236]
};

pub const IID_ID3D11TEXTURE2D: GUID = GUID {
    Data1: 1863690994,
    Data2: 53768,
    Data3: 20105,
    Data4: [154, 180, 72, 149, 53, 211, 79, 156]
};

#[link(name="dxgi")]
#[link(name="d3d11")]
extern "system" {
    pub fn CreateDXGIFactory1(
        id: REFIID,
        pp_factory: *mut *mut IDXGIFactory1
    ) -> HRESULT;

    pub fn D3D11CreateDevice(
        pAdapter: *mut IDXGIAdapter1,
        DriverType: D3D_DRIVER_TYPE,
        Software: HMODULE,
        Flags: UINT,
        pFeatureLevels: *mut D3D_FEATURE_LEVEL,
        FeatureLevels: UINT,
        SDKVersion: UINT,
        ppDevice: *mut *mut ID3D11Device,
        pFeatureLevel: *mut D3D_FEATURE_LEVEL,
        ppImmediateContext: *mut *mut ID3D11DeviceContext
    ) -> HRESULT;
}
