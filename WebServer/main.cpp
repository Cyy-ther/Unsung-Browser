#define NOMINMAX
#include <iostream>
#include <string>
#include <sstream>
#include <algorithm>
#include <vector>
#include <regex>
#include <chrono>
#include <unordered_map>
#include <mutex>
#include <winsock2.h>
#include <windows.h>
#include <wininet.h>
#include <ws2tcpip.h>

#pragma comment(lib, "ws2_32.lib")
#pragma comment(lib, "wininet.lib")

const int MAX_REQUESTS_PER_MINUTE = 120;
const int MAX_RESPONSE_SIZE = 50 * 1024 * 1024;
const int REQUEST_TIMEOUT_MS = 60000;

class Base64 {
public:
    static std::string encode(const std::string& input) {
        static const char* base64_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        std::string ret;
        int i = 0;
        unsigned char char_array_3[3];
        unsigned char char_array_4[4];
        for (size_t n = 0; n < input.length(); n++) {
            char_array_3[i++] = input[n];
            if (i == 3) {
                char_array_4[0] = (char_array_3[0] & 0xfc) >> 2;
                char_array_4[1] = ((char_array_3[0] & 0x03) << 4) + ((char_array_3[1] & 0xf0) >> 4);
                char_array_4[2] = ((char_array_3[1] & 0x0f) << 2) + ((char_array_3[2] & 0xc0) >> 6);
                char_array_4[3] = char_array_3[2] & 0x3f;
                for(i = 0; i < 4; i++) ret += base64_chars[char_array_4[i]];
                i = 0;
            }
        }
        if (i) {
            for(int j = i; j < 3; j++) char_array_3[j] = '\0';
            char_array_4[0] = (char_array_3[0] & 0xfc) >> 2;
            char_array_4[1] = ((char_array_3[0] & 0x03) << 4) + ((char_array_3[1] & 0xf0) >> 4);
            char_array_4[2] = ((char_array_3[1] & 0x0f) << 2) + ((char_array_3[2] & 0xc0) >> 6);
            for (int j = 0; j < i + 1; j++) ret += base64_chars[char_array_4[j]];
        }
        return ret;
    }

    static std::string decode(const std::string& input) {
        static const std::string base64_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        std::string ret;
        std::vector<int> T(256, -1);
        for (int i = 0; i < 64; i++) T[base64_chars[i]] = i;
        int val = 0, valb = -8;
        for (unsigned char c : input) {
            if (T[c] == -1) break;
            val = (val << 6) + T[c];
            valb += 6;
            if (valb >= 0) {
                ret.push_back(char((val >> valb) & 0xFF));
                valb -= 8;
            }
        }
        return ret;
    }
};

class RateLimiter {
private:
    struct ClientInfo { int request_count; std::chrono::steady_clock::time_point window_start; };
    std::unordered_map<std::string, ClientInfo> clients;
    std::mutex mtx;
public:
    bool allowRequest(const std::string& client_ip) {
        std::lock_guard<std::mutex> lock(mtx);
        auto now = std::chrono::steady_clock::now();
        auto& info = clients[client_ip];
        auto elapsed = std::chrono::duration_cast<std::chrono::minutes>(now - info.window_start);
        if (elapsed.count() >= 1) { info.request_count = 0; info.window_start = now; }
        if (info.request_count >= MAX_REQUESTS_PER_MINUTE) return false;
        info.request_count++;
        return true;
    }
};

class HTTPFetcher {
public:
    struct FetchResult { std::string content, content_type, error; long status_code; bool success; };
    FetchResult fetch(const std::string& url) {
        FetchResult result; result.success = false; result.status_code = 0; result.content_type = "text/html";
        HINTERNET hInternet = InternetOpenA("Mozilla/5.0", INTERNET_OPEN_TYPE_DIRECT, NULL, NULL, 0);
        if (!hInternet) { result.error = "Failed to initialize"; return result; }
        DWORD timeout = REQUEST_TIMEOUT_MS;
        InternetSetOptionA(hInternet, INTERNET_OPTION_CONNECT_TIMEOUT, &timeout, sizeof(timeout));
        HINTERNET hConnect = InternetOpenUrlA(hInternet, url.c_str(), NULL, 0, INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE, 0);
        if (!hConnect) { result.error = "Failed to open URL"; InternetCloseHandle(hInternet); return result; }
        char contentType[256]; DWORD contentTypeSize = sizeof(contentType);
        if (HttpQueryInfoA(hConnect, HTTP_QUERY_CONTENT_TYPE, contentType, &contentTypeSize, NULL)) result.content_type = std::string(contentType);
        char buffer[8192]; DWORD bytesRead; std::string response; size_t total_size = 0;
        while (InternetReadFile(hConnect, buffer, sizeof(buffer), &bytesRead) && bytesRead > 0) {
            total_size += bytesRead;
            if (total_size > MAX_RESPONSE_SIZE) { result.error = "Response too large"; InternetCloseHandle(hConnect); InternetCloseHandle(hInternet); return result; }
            response.append(buffer, bytesRead);
        }
        result.content = response; result.status_code = 200; result.success = true;
        InternetCloseHandle(hConnect); InternetCloseHandle(hInternet);
        return result;
    }
};

class URLRewriter {
private:
    std::string proxy_base;
    std::string getOrigin(const std::string& url) {
        size_t proto_end = url.find("://");
        if (proto_end == std::string::npos) return "";
        size_t path_start = url.find('/', proto_end + 3);
        return path_start == std::string::npos ? url : url.substr(0, path_start);
    }

    std::string resolveURL(const std::string& base, const std::string& relative) {
        if (relative.find("http://") == 0 || relative.find("https://") == 0) return relative;
        if (relative.find("//") == 0) return "https:" + relative;
        if (relative.find("/") == 0) return getOrigin(base) + relative;
        size_t last_slash = base.rfind('/');
        return last_slash != std::string::npos ? base.substr(0, last_slash + 1) + relative : base + "/" + relative;
    }

public:
    URLRewriter(const std::string& proxy) : proxy_base(proxy) {}

    std::string rewrite(const std::string& html, const std::string& base_url) {
        std::string result = html;
        std::regex url_regex(R"((https?://[^\"'\s<>]+))");
        std::smatch match;
        std::string::const_iterator start = result.cbegin();
        std::vector<std::pair<size_t, std::pair<size_t, std::string>>> replacements;

        while (std::regex_search(start, result.cend(), match, url_regex)) {
            std::string original_url = match[1].str();
            std::string encoded = Base64::encode(original_url);
            std::string proxy_url = proxy_base + "/proxy/" + encoded;
            size_t pos = match.position(0) + (start - result.cbegin());
            replacements.push_back({pos, {original_url.length(), proxy_url}});
            start = match.suffix().first;
        }

        for (auto it = replacements.rbegin(); it != replacements.rend(); ++it) {
            result.replace(it->first, it->second.first, it->second.second);
        }

        size_t head_pos = result.find("<head>");
        if (head_pos != std::string::npos) {
            std::string base_tag = "<base href=\"" + proxy_base + "/proxy/" + Base64::encode(base_url) + "/\">";
            result.insert(head_pos + 6, base_tag);
        }

        return result;
    }
};

class HTTPServer {
private:
    SOCKET server_socket;
    int port;
    HTTPFetcher fetcher;
    RateLimiter rate_limiter;
    URLRewriter rewriter;

    void sendResponse(SOCKET client, int status, const std::string& contentType, const std::string& body) {
        std::string statusStr = (status == 200) ? "200 OK" : (status == 429) ? "429 Too Many Requests" : (status == 400) ? "400 Bad Request" : "502 Bad Gateway";
        std::stringstream ss;
        ss << "HTTP/1.1 " << statusStr << "\r\n";
        ss << "Content-Type: " << contentType << "\r\n";
        ss << "Content-Length: " << body.length() << "\r\n";
        ss << "Access-Control-Allow-Origin: *\r\n";
        ss << "Connection: close\r\n\r\n";
        ss << body;
        std::string fullResponse = ss.str();
        send(client, fullResponse.c_str(), (int)fullResponse.length(), 0);
    }

    std::string escapeJSON(const std::string& input) {
        std::string result;
        for (char c : input) {
            if (c == '"') result += "\\\"";
            else if (c == '\\') result += "\\\\";
            else if (c == '\n') result += "\\n";
            else if (c == '\r') result += "\\r";
            else if (c == '\t') result += "\\t";
            else if (c == '\b') result += "\\b";
            else if (c == '\f') result += "\\f";
            else result += c;
        }
        return result;
    }

    void handleClient(SOCKET client_socket) {
        char buffer[16384] = {0};
        int bytes_received = recv(client_socket, buffer, sizeof(buffer) - 1, 0);
        if (bytes_received <= 0) { closesocket(client_socket); return; }

        std::string request(buffer, bytes_received);
        std::string method, path;
        size_t s1 = request.find(' '), s2 = request.find(' ', s1 + 1);
        if (s1 != std::string::npos && s2 != std::string::npos) {
            method = request.substr(0, s1);
            path = request.substr(s1 + 1, s2 - s1 - 1);
        }

        std::cout << "\n=== NEW REQUEST ===" << std::endl;
        std::cout << "Method: " << method << std::endl;
        std::cout << "Path: " << path << std::endl;

        if (!rate_limiter.allowRequest("client_ip")) {
            std::cout << "BLOCKED: Rate limit" << std::endl;
            sendResponse(client_socket, 429, "application/json", "{\"error\":\"Rate limit exceeded\"}");
            closesocket(client_socket); return;
        }

        if (method == "POST" && path == "/fetch") {
            size_t body_start = request.find("\r\n\r\n");
            if (body_start == std::string::npos) {
                std::cout << "ERROR: No request body" << std::endl;
                sendResponse(client_socket, 400, "application/json", "{\"error\":\"No request body\"}");
                closesocket(client_socket); return;
            }
            std::string body = request.substr(body_start + 4);
            std::cout << "Request body: " << body << std::endl;

            size_t url_start = body.find("\"url\":\"");
            if (url_start == std::string::npos) {
                std::cout << "ERROR: Missing url field" << std::endl;
                sendResponse(client_socket, 400, "application/json", "{\"error\":\"Missing url field\"}");
                closesocket(client_socket); return;
            }
            url_start += 7;
            size_t url_end = body.find("\"", url_start);
            std::string url = body.substr(url_start, url_end - url_start);

            std::cout << "Fetching: " << url << std::endl;
            auto result = fetcher.fetch(url);

            if (result.success) {
                std::cout << "SUCCESS: Got " << result.content.length() << " bytes" << std::endl;
                std::string escaped_content = escapeJSON(result.content);
                std::stringstream json;
                json << "{\"success\":true,\"content\":\"" << escaped_content
                     << "\",\"contentType\":\"" << result.content_type << "\"}";
                sendResponse(client_socket, 200, "application/json", json.str());
            } else {
                std::cout << "FAILED: " << result.error << std::endl;
                sendResponse(client_socket, 502, "application/json",
                           "{\"success\":false,\"error\":\"" + result.error + "\"}");
            }
        }
        else if (path.find("/proxy/") == 0) {
            std::string encoded_url = path.substr(7);
            size_t slash_pos = encoded_url.find('/');
            if (slash_pos != std::string::npos) encoded_url = encoded_url.substr(0, slash_pos);

            std::cout << "Encoded URL: " << encoded_url << std::endl;

            std::string url;
            try {
                url = Base64::decode(encoded_url);
                std::cout << "Decoded URL: " << url << std::endl;
            } catch (...) {
                std::cout << "ERROR: Failed to decode Base64" << std::endl;
                sendResponse(client_socket, 400, "text/plain", "Invalid encoding");
                closesocket(client_socket); return;
            }

            if (url.find("http://") != 0 && url.find("https://") != 0) {
                std::cout << "ERROR: URL missing http/https" << std::endl;
                sendResponse(client_socket, 400, "text/plain", "Invalid URL - must start with http:// or https://");
                closesocket(client_socket); return;
            }

            std::cout << "Fetching: " << url << std::endl;
            auto result = fetcher.fetch(url);

            if (result.success) {
                std::cout << "SUCCESS: Got " << result.content.length() << " bytes" << std::endl;
                std::cout << "Content-Type: " << result.content_type << std::endl;

                std::string content = result.content;
                if (result.content_type.find("text/html") != std::string::npos) {
                    std::cout << "Rewriting HTML..." << std::endl;
                    content = rewriter.rewrite(content, url);
                }

                sendResponse(client_socket, 200, result.content_type, content);
            } else {
                std::cout << "FAILED: " << result.error << std::endl;
                sendResponse(client_socket, 502, "text/plain", "Failed to fetch: " + result.error);
            }
        }
        else if (path.find("/navigate?url=") == 0) {
            std::string url = path.substr(14);
            std::string decoded_url;
            for (size_t i = 0; i < url.length(); i++) {
                if (url[i] == '%' && i + 2 < url.length()) {
                    int value;
                    sscanf(url.substr(i + 1, 2).c_str(), "%x", &value);
                    decoded_url += static_cast<char>(value);
                    i += 2;
                } else if (url[i] == '+') {
                    decoded_url += ' ';
                } else {
                    decoded_url += url[i];
                }
            }
            std::cout << "Redirecting to: " << decoded_url << std::endl;
            std::string encoded = Base64::encode(decoded_url);
            std::string response = "HTTP/1.1 302 Found\r\nLocation: /proxy/" + encoded + "\r\n\r\n";
            send(client_socket, response.c_str(), response.length(), 0);
        }
        else {
            std::cout << "Serving home page" << std::endl;
            std::string home = "<!DOCTYPE html><html><head><title>Cypher Proxy</title></head><body>"
                             "<h1>Cypher Web Proxy</h1>"
                             "<form action=\"/navigate\" method=\"get\">"
                             "<input type=\"text\" name=\"url\" placeholder=\"Enter URL (e.g., https://example.com)\" style=\"width:500px\">"
                             "<button type=\"submit\">Go</button>"
                             "</form></body></html>";
            sendResponse(client_socket, 200, "text/html", home);
        }
        closesocket(client_socket);
    }

public:
    HTTPServer(int port) : port(port), server_socket(INVALID_SOCKET), rewriter("http://localhost:" + std::to_string(port)) {
        WSADATA wsa_data; WSAStartup(MAKEWORD(2, 2), &wsa_data);
    }
    ~HTTPServer() { if (server_socket != INVALID_SOCKET) closesocket(server_socket); WSACleanup(); }
    void run() {
        server_socket = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
        sockaddr_in address{}; address.sin_family = AF_INET; address.sin_addr.s_addr = INADDR_ANY; address.sin_port = htons(port);
        bind(server_socket, (struct sockaddr*)&address, sizeof(address));
        listen(server_socket, 10);
        std::cout << "Server running on http://localhost:" << port << std::endl;
        while (true) { SOCKET client = accept(server_socket, nullptr, nullptr); if (client != INVALID_SOCKET) handleClient(client); }
    }
};

int main() { HTTPServer server(8080); server.run(); return 0; }
