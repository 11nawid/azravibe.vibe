/* Azra native GUI core — Win32 widgets with a C ABI. */
#ifndef AZRA_GUI_H
#define AZRA_GUI_H

#ifdef __cplusplus
extern "C" {
#endif

long long azra_gui_window(const char *title, int width, int height);
int azra_gui_label(long long window, const char *text, int x, int y, int w, int h);
int azra_gui_button(long long window, const char *text, int x, int y, int w, int h);
int azra_gui_edit(long long window, const char *text, int x, int y, int w, int h);
int azra_gui_set_text(long long window, int id, const char *text);
int azra_gui_get_text(long long window, int id, char *out, int capacity);
int azra_gui_show(long long window);
/* Non-blocking message pump; writes {"type":..,"id":..,"text":".."} to out. */
int azra_gui_pump(long long window, char *out, int capacity);
int azra_gui_message(const char *text, const char *title);
int azra_gui_close(long long window);

#ifdef __cplusplus
}
#endif
#endif
