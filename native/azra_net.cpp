// Azra native networking core: a small HTTP/1.1 server and client with a C ABI.
// No third-party libraries. Windows uses Winsock; other platforms use BSD sockets.
#include "azra_net.h"

#define NOMINMAX
#include <algorithm>
#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <map>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
using sock_t = SOCKET;
static const sock_t BAD_SOCK = INVALID_SOCKET;
static void sock_close(sock_t s) { closesocket(s); }
static bool net_init() {
    static bool ready = false;
    if (!ready) {
        WSADATA data;
        if (WSAStartup(MAKEWORD(2, 2), &data) != 0) return false;
        ready = true;
    }
    return true;
}
#else
#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
using sock_t = int;
static const sock_t BAD_SOCK = -1;
static void sock_close(sock_t s) { close(s); }
static bool net_init() { return true; }
#endif

namespace {

std::string json_escape(const std::string &value) {
    std::string out;
    out.reserve(value.size() + 8);
    for (unsigned char c : value) {
        switch (c) {
        case '"': out += "\\\""; break;
        case '\\': out += "\\\\"; break;
        case '\n': out += "\\n"; break;
        case '\r': out += "\\r"; break;
        case '\t': out += "\\t"; break;
        default:
            if (c < 0x20) {
                char buf[8];
                std::snprintf(buf, sizeof(buf), "\\u%04x", c);
                out += buf;
            } else {
                out += static_cast<char>(c);
            }
        }
    }
    return out;
}

std::string lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(), [](unsigned char c) {
        return static_cast<char>(std::tolower(c));
    });
    return s;
}

void send_all(sock_t socket, const std::string &text) {
    size_t sent = 0;
    while (sent < text.size()) {
        int n = static_cast<int>(std::min<size_t>(text.size() - sent, 8192));
        int wrote = send(socket, text.data() + sent, n, 0);
        if (wrote <= 0) break;
        sent += static_cast<size_t>(wrote);
    }
}

struct Pending {
    long long id;
    std::string json;
    sock_t socket;
};

struct Server {
    sock_t listener = BAD_SOCK;
    std::mutex mutex;
    std::vector<Pending> queue;
    std::map<long long, sock_t> sockets;
    long long next_id = 1;
    bool running = true;
    std::thread worker;
};

std::mutex servers_mutex;
std::map<long long, Server *> servers;

std::string read_request(sock_t client) {
    std::string raw;
    char buffer[4096];
    while (raw.find("\r\n\r\n") == std::string::npos) {
        int n = recv(client, buffer, sizeof(buffer), 0);
        if (n <= 0) return std::string();
        raw.append(buffer, static_cast<size_t>(n));
        if (raw.size() > 16 * 1024 * 1024) return std::string();
    }
    size_t header_end = raw.find("\r\n\r\n") + 4;
    size_t content_length = 0;
    std::istringstream lines(raw.substr(0, header_end));
    std::string line;
    while (std::getline(lines, line)) {
        auto colon = line.find(':');
        if (colon == std::string::npos) continue;
        std::string key = lower(line.substr(0, colon));
        if (key == "content-length") {
            content_length = std::strtoul(line.substr(colon + 1).c_str(), nullptr, 10);
        }
    }
    while (raw.size() - header_end < content_length) {
        int n = recv(client, buffer, sizeof(buffer), 0);
        if (n <= 0) break;
        raw.append(buffer, static_cast<size_t>(n));
    }
    return raw;
}

void server_loop(Server *server) {
    while (server->running) {
        sock_t client = accept(server->listener, nullptr, nullptr);
        if (client == BAD_SOCK) break;
        std::string raw = read_request(client);
        if (raw.empty()) {
            sock_close(client);
            continue;
        }
        size_t header_end = raw.find("\r\n\r\n");
        std::string head = raw.substr(0, header_end);
        std::string body = raw.substr(header_end + 4);
        std::string method, target;
        std::istringstream first(head);
        std::getline(first, method, ' ');
        std::getline(first, target, ' ');
        size_t q = target.find('?');
        std::string path = q == std::string::npos ? target : target.substr(0, q);
        std::string query = q == std::string::npos ? "" : target.substr(q + 1);
        std::string headers = head.substr(head.find('\n') + 1);
        while (!headers.empty() && headers.back() == '\r') headers.pop_back();

        long long id;
        {
            std::lock_guard<std::mutex> guard(server->mutex);
            id = server->next_id++;
            server->sockets[id] = client;
        }
        std::string json = "{\"id\":" + std::to_string(id) +
            ",\"method\":\"" + json_escape(method) +
            "\",\"path\":\"" + json_escape(path) +
            "\",\"query\":\"" + json_escape(query) +
            "\",\"headers\":\"" + json_escape(headers) +
            "\",\"body\":\"" + json_escape(body) + "\"}";
        {
            std::lock_guard<std::mutex> guard(server->mutex);
            server->queue.push_back(Pending{id, json, client});
        }
    }
}

std::string status_text(int status) {
    switch (status) {
    case 200: return "OK";
    case 201: return "Created";
    case 204: return "No Content";
    case 301: return "Moved Permanently";
    case 400: return "Bad Request";
    case 401: return "Unauthorized";
    case 403: return "Forbidden";
    case 404: return "Not Found";
    case 405: return "Method Not Allowed";
    case 500: return "Internal Server Error";
    default: return "Status";
    }
}

bool parse_url(const std::string &url, std::string &host, int &port, std::string &path) {
    std::string rest = url;
    size_t scheme = rest.find("://");
    if (scheme != std::string::npos) rest = rest.substr(scheme + 3);
    size_t slash = rest.find('/');
    std::string authority = slash == std::string::npos ? rest : rest.substr(0, slash);
    path = slash == std::string::npos ? "/" : rest.substr(slash);
    size_t colon = authority.find(':');
    host = colon == std::string::npos ? authority : authority.substr(0, colon);
    port = colon == std::string::npos ? 80 : std::atoi(authority.c_str() + colon + 1);
    return !host.empty();
}

} // namespace

extern "C" {

long long azra_net_listen(int port) {
    if (!net_init()) return -1;
    sock_t listener = socket(AF_INET, SOCK_STREAM, 0);
    if (listener == BAD_SOCK) return -1;
    int yes = 1;
    setsockopt(listener, SOL_SOCKET, SO_REUSEADDR, reinterpret_cast<const char *>(&yes), sizeof(yes));
    sockaddr_in address;
    std::memset(&address, 0, sizeof(address));
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_ANY);
    address.sin_port = htons(static_cast<unsigned short>(port));
    if (bind(listener, reinterpret_cast<sockaddr *>(&address), sizeof(address)) != 0 ||
        listen(listener, 64) != 0) {
        sock_close(listener);
        return -1;
    }
    Server *server = new Server();
    server->listener = listener;
    server->worker = std::thread(server_loop, server);
    std::lock_guard<std::mutex> guard(servers_mutex);
    long long handle = static_cast<long long>(servers.size());
    while (servers.count(handle)) ++handle;
    servers[handle] = server;
    return handle;
}

int azra_net_poll(long long handle, char *out, int capacity) {
    if (!out || capacity <= 0) return -1;
    Server *server = nullptr;
    {
        std::lock_guard<std::mutex> guard(servers_mutex);
        auto it = servers.find(handle);
        if (it == servers.end()) return -1;
        server = it->second;
    }
    std::lock_guard<std::mutex> guard(server->mutex);
    if (server->queue.empty()) return 0;
    Pending pending = server->queue.front();
    server->queue.erase(server->queue.begin());
    int n = static_cast<int>(std::min<size_t>(pending.json.size(), static_cast<size_t>(capacity - 1)));
    std::memcpy(out, pending.json.data(), static_cast<size_t>(n));
    out[n] = '\0';
    return 1;
}

int azra_net_respond(long long handle, long long request_id, int status,
                     const char *content_type, const char *body, int body_len) {
    Server *server = nullptr;
    {
        std::lock_guard<std::mutex> guard(servers_mutex);
        auto it = servers.find(handle);
        if (it == servers.end()) return -1;
        server = it->second;
    }
    sock_t client = BAD_SOCK;
    {
        std::lock_guard<std::mutex> guard(server->mutex);
        auto it = server->sockets.find(request_id);
        if (it == server->sockets.end()) return -1;
        client = it->second;
        server->sockets.erase(it);
    }
    std::string text = content_type ? content_type : "text/plain; charset=utf-8";
    std::string payload = body ? std::string(body, static_cast<size_t>(std::max(0, body_len))) : std::string();
    std::string response = "HTTP/1.1 " + std::to_string(status) + " " + status_text(status) +
        "\r\nContent-Type: " + text +
        "\r\nContent-Length: " + std::to_string(payload.size()) +
        "\r\nConnection: close\r\n\r\n" + payload;
    send_all(client, response);
    sock_close(client);
    return 0;
}

int azra_net_request(const char *method, const char *url, const char *headers,
                     const char *body, char *out, int capacity) {
    if (!method || !url || !out || capacity <= 0) return -1;
    if (!net_init()) return -1;
    std::string host, path;
    int port = 80;
    if (!parse_url(url, host, port, path)) return -1;
    addrinfo hints;
    std::memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    addrinfo *addresses = nullptr;
    if (getaddrinfo(host.c_str(), std::to_string(port).c_str(), &hints, &addresses) != 0) return -1;
    sock_t socket = BAD_SOCK;
    for (addrinfo *addr = addresses; addr; addr = addr->ai_next) {
        socket = ::socket(addr->ai_family, addr->ai_socktype, addr->ai_protocol);
        if (socket != BAD_SOCK && connect(socket, addr->ai_addr, static_cast<int>(addr->ai_addrlen)) == 0) break;
        if (socket != BAD_SOCK) { sock_close(socket); socket = BAD_SOCK; }
    }
    freeaddrinfo(addresses);
    if (socket == BAD_SOCK) return -1;
    std::string payload = body ? body : "";
    std::string request = std::string(method) + " " + path + " HTTP/1.1\r\nHost: " + host +
        "\r\nConnection: close\r\n";
    if (headers && *headers) request += headers;
    if (request.find("Content-Length:") == std::string::npos && !payload.empty()) {
        request += "Content-Length: " + std::to_string(payload.size()) + "\r\n";
    }
    request += "\r\n" + payload;
    send_all(socket, request);
    std::string raw;
    char buffer[4096];
    int n;
    while ((n = recv(socket, buffer, sizeof(buffer), 0)) > 0) raw.append(buffer, static_cast<size_t>(n));
    sock_close(socket);
    size_t split = raw.find("\r\n\r\n");
    if (split == std::string::npos) return -1;
    std::istringstream first(raw);
    std::string version;
    int status = 0;
    first >> version >> status;
    std::string response_headers = raw.substr(raw.find('\n') + 1, split - raw.find('\n'));
    std::string response_body = raw.substr(split + 4);
    std::string json = "{\"status\":" + std::to_string(status) +
        ",\"headers\":\"" + json_escape(response_headers) +
        "\",\"body\":\"" + json_escape(response_body) + "\"}";
    int written = static_cast<int>(std::min<size_t>(json.size(), static_cast<size_t>(capacity - 1)));
    std::memcpy(out, json.data(), static_cast<size_t>(written));
    out[written] = '\0';
    return status;
}

int azra_net_close(long long handle) {
    Server *server = nullptr;
    {
        std::lock_guard<std::mutex> guard(servers_mutex);
        auto it = servers.find(handle);
        if (it == servers.end()) return -1;
        server = it->second;
        servers.erase(it);
    }
    server->running = false;
    shutdown(server->listener, 2);
    sock_close(server->listener);
    if (server->worker.joinable()) server->worker.join();
    std::lock_guard<std::mutex> guard(server->mutex);
    for (auto &entry : server->sockets) sock_close(entry.second);
    delete server;
    return 0;
}

} // extern "C"
