/* Azra native networking core — C ABI shared with the Rust FFI layer. */
#ifndef AZRA_NET_H
#define AZRA_NET_H

#ifdef __cplusplus
extern "C" {
#endif

/* Negative values are errors; handles are non-negative. */
long long azra_net_listen(int port);
int azra_net_poll(long long server, char *out, int capacity);
int azra_net_respond(long long server, long long request_id, int status,
                     const char *content_type, const char *body, int body_len);
/* Returns HTTP status (>0) or -1 on error; `out` receives a JSON response. */
int azra_net_request(const char *method, const char *url, const char *headers,
                     const char *body, char *out, int capacity);
int azra_net_close(long long server);

#ifdef __cplusplus
}
#endif
#endif
