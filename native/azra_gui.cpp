// Native Win32 GUI core for Azra. On non-Windows platforms the API returns
// harmless errors so the crate still compiles and links everywhere.
#include "azra_gui.h"

#include <algorithm>
#include <map>
#include <mutex>
#include <string>
#include <vector>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

namespace {

int next_id = 1;
std::string json_escape(const std::string &value) {
    std::string out;
    for (unsigned char c : value) {
        switch (c) {
        case '"': out += "\\\""; break;
        case '\\': out += "\\\\"; break;
        case '\n': out += "\\n"; break;
        case '\r': out += "\\r"; break;
        default: out += static_cast<char>(c);
        }
    }
    return out;
}

struct Event {
    int type;
    int id;
    std::string text;
};

struct Window {
    HWND hwnd = nullptr;
    std::map<int, HWND> controls;
    std::vector<Event> events;
    std::mutex mutex;
};

std::mutex registry_mutex;
std::map<long long, Window *> registry;

Window *find_window(long long handle) {
    std::lock_guard<std::mutex> guard(registry_mutex);
    auto it = registry.find(handle);
    return it == registry.end() ? nullptr : it->second;
}

LRESULT CALLBACK window_proc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
    Window *window = reinterpret_cast<Window *>(GetWindowLongPtr(hwnd, GWLP_USERDATA));
    if (message == WM_COMMAND && window) {
        int id = static_cast<int>(LOWORD(wparam));
        std::string text;
        auto it = window->controls.find(id);
        if (it != window->controls.end()) {
            char buffer[1024];
            int n = GetWindowTextA(it->second, buffer, sizeof(buffer) - 1);
            if (n > 0) text.assign(buffer, static_cast<size_t>(n));
        }
        std::lock_guard<std::mutex> guard(window->mutex);
        window->events.push_back(Event{1, id, text});
        return 0;
    }
    if (message == WM_CLOSE && window) {
        std::lock_guard<std::mutex> guard(window->mutex);
        window->events.push_back(Event{3, 0, "closed"});
        DestroyWindow(hwnd);
        return 0;
    }
    return DefWindowProc(hwnd, message, wparam, lparam);
}

HWND make_control(Window *window, const char *kind, const char *text,
                  int id, int x, int y, int w, int h) {
    HWND control = CreateWindowExA(
        0, kind, text ? text : "",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL,
        x, y, w, h, window->hwnd, reinterpret_cast<HMENU>(static_cast<intptr_t>(id)),
        nullptr, nullptr);
    if (control) window->controls[id] = control;
    return control;
}

} // namespace

extern "C" {

long long azra_gui_window(const char *title, int width, int height) {
    static bool registered = false;
    if (!registered) {
        WNDCLASSA wc;
        ZeroMemory(&wc, sizeof(wc));
        wc.lpfnWndProc = window_proc;
        wc.hInstance = GetModuleHandle(nullptr);
        wc.lpszClassName = "AzraNativeWindow";
        wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
        wc.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_WINDOW + 1);
        if (!RegisterClassA(&wc)) return -1;
        registered = true;
    }
    Window *window = new Window();
    window->hwnd = CreateWindowExA(
        0, "AzraNativeWindow", title ? title : "Azra",
        WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, width, height,
        nullptr, nullptr, GetModuleHandle(nullptr), nullptr);
    if (!window->hwnd) {
        delete window;
        return -1;
    }
    SetWindowLongPtr(window->hwnd, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(window));
    long long handle;
    {
        std::lock_guard<std::mutex> guard(registry_mutex);
        handle = static_cast<long long>(registry.size());
        while (registry.count(handle)) ++handle;
        registry[handle] = window;
    }
    return handle;
}

int azra_gui_label(long long handle, const char *text, int x, int y, int w, int h) {
    Window *window = find_window(handle);
    if (!window) return -1;
    int id = next_id++;
    if (!make_control(window, "STATIC", text, id, x, y, w, h)) return -1;
    return id;
}

int azra_gui_button(long long handle, const char *text, int x, int y, int w, int h) {
    Window *window = find_window(handle);
    if (!window) return -1;
    int id = next_id++;
    if (!make_control(window, "BUTTON", text, id, x, y, w, h)) return -1;
    return id;
}

int azra_gui_edit(long long handle, const char *text, int x, int y, int w, int h) {
    Window *window = find_window(handle);
    if (!window) return -1;
    int id = next_id++;
    if (!make_control(window, "EDIT", text, id, x, y, w, h)) return -1;
    return id;
}

int azra_gui_set_text(long long handle, int id, const char *text) {
    Window *window = find_window(handle);
    if (!window) return -1;
    auto it = window->controls.find(id);
    if (it == window->controls.end()) return -1;
    SetWindowTextA(it->second, text ? text : "");
    return 0;
}

int azra_gui_get_text(long long handle, int id, char *out, int capacity) {
    Window *window = find_window(handle);
    if (!window || !out || capacity <= 0) return -1;
    auto it = window->controls.find(id);
    if (it == window->controls.end()) return -1;
    int n = GetWindowTextA(it->second, out, capacity - 1);
    if (n > 0) out[n] = '\0';
    return n;
}

int azra_gui_show(long long handle) {
    Window *window = find_window(handle);
    if (!window) return -1;
    ShowWindow(window->hwnd, SW_SHOW);
    UpdateWindow(window->hwnd);
    return 0;
}

int azra_gui_pump(long long handle, char *out, int capacity) {
    Window *window = find_window(handle);
    if (!window || !out || capacity <= 0) return -1;
    MSG message;
    while (PeekMessageA(&message, nullptr, 0, 0, PM_REMOVE)) {
        TranslateMessage(&message);
        DispatchMessageA(&message);
    }
    std::lock_guard<std::mutex> guard(window->mutex);
    if (window->events.empty()) return IsWindow(window->hwnd) ? 0 : 2;
    Event event = window->events.front();
    window->events.erase(window->events.begin());
    std::string json = "{\"type\":" + std::to_string(event.type) +
        ",\"id\":" + std::to_string(event.id) +
        ",\"text\":\"" + json_escape(event.text) + "\"}";
    int n = static_cast<int>(std::min<size_t>(json.size(), static_cast<size_t>(capacity - 1)));
    std::copy(json.begin(), json.begin() + n, out);
    out[n] = '\0';
    return 1;
}

int azra_gui_message(const char *text, const char *title) {
    MessageBoxA(nullptr, text ? text : "", title ? title : "Azra", MB_OK);
    return 0;
}

int azra_gui_close(long long handle) {
    Window *window = find_window(handle);
    if (!window) return -1;
    {
        std::lock_guard<std::mutex> guard(registry_mutex);
        registry.erase(handle);
    }
    if (window->hwnd) DestroyWindow(window->hwnd);
    delete window;
    return 0;
}

} // extern "C"

#else // !_WIN32

extern "C" {

long long azra_gui_window(const char *, int, int) { return -1; }
int azra_gui_label(long long, const char *, int, int, int, int) { return -1; }
int azra_gui_button(long long, const char *, int, int, int, int) { return -1; }
int azra_gui_edit(long long, const char *, int, int, int, int) { return -1; }
int azra_gui_set_text(long long, int, const char *) { return -1; }
int azra_gui_get_text(long long, int, char *out, int capacity) {
    if (out && capacity > 0) out[0] = '\0';
    return -1;
}
int azra_gui_show(long long) { return -1; }
int azra_gui_pump(long long, char *out, int capacity) {
    if (out && capacity > 0) out[0] = '\0';
    return -1;
}
int azra_gui_message(const char *, const char *) { return -1; }
int azra_gui_close(long long) { return -1; }

} // extern "C"

#endif
