// C++ valid nested brackets
// Tests: raw strings, block/line comments, templates, nested containers

#include <vector>
#include <map>
#include <string>

// Raw string — braces inside must NOT be counted
const char* raw = R"delim(
    these { braces [ inside ] raw } strings must be ignored
)delim";

/* Block comment { [ ( should be ignored ) ] } */

template<typename K, typename V>
struct Cache {
    std::map<K, std::vector<V>> data;

    void insert(K key, std::vector<V> vals) {
        data[key] = std::move(vals);
    }

    std::vector<V>* find(const K& key) {
        auto it = data.find(key);
        if (it != data.end()) {
            return &it->second;
        }
        return nullptr;
    }
};

int main() {
    Cache<std::string, int> c;
    c.insert("a", {1, 2, 3});
    c.insert("b", {4, {5}, 6});

    auto nested = std::map<std::string, std::map<int, std::vector<int>>>{
        {"x", {{1, {10, 20}}, {2, {30, 40}}}},
        {"y", {{3, {50}}}},
    };

    for (auto& [k, inner] : nested) {
        for (auto& [n, vals] : inner) {
            (void)k;
            (void)n;
            (void)vals;
        }
    }

    return 0;
}
