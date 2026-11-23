#include <windows.h>
#include <shellapi.h>
#include <shobjidl.h>
#include <propkey.h>
#include <propvarutil.h>
#include <string>

// Shell grouping and pinning are independent of the window's themed small icon.
extern "C" HRESULT configure_settings_taskbar(HWND hwnd, const wchar_t* executable) {
    IPropertyStore* store = nullptr;
    HRESULT hr = SHGetPropertyStoreForWindow(hwnd, IID_PPV_ARGS(&store));
    if (FAILED(hr)) return hr;
    const auto put = [&](REFPROPERTYKEY key, const std::wstring& value) {
        PROPVARIANT prop{};
        HRESULT result = InitPropVariantFromString(value.c_str(), &prop);
        if (SUCCEEDED(result)) result = store->SetValue(key, prop);
        PropVariantClear(&prop);
        return result;
    };
    const std::wstring exe(executable);
    hr = put(PKEY_AppUserModel_RelaunchCommand, L"\"" + exe + L"\" --settings-only");
    if (SUCCEEDED(hr)) hr = put(PKEY_AppUserModel_RelaunchDisplayNameResource, L"@" + exe + L",-401");
    if (SUCCEEDED(hr)) hr = put(PKEY_AppUserModel_RelaunchIconResource, exe + L",-41000");
    if (SUCCEEDED(hr)) hr = put(PKEY_AppUserModel_ID, L"Tanveer.StackDockNative.Settings");
    if (SUCCEEDED(hr)) hr = store->Commit();
    store->Release();
    return hr;
}
