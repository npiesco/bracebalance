// C++ broken nested brackets
// Tests: raw strings contain decoy braces that must be stripped

#include <vector>
#include <map>
#include <string>

// Raw string with decoy braces — must be ignored by sanitizer
const char* trap = R"END(
    fake { unclosed [ bracket in raw string
)END";

/* Another decoy: { [ ( inside block comment ) ] } */

template<typename T>
struct BrokenContainer {
    std::vector<std::map<std::string, std::vector<T>>> layers;

    void push(std::string key, std::vector<T> vals) {
        layers.push_back({{key, std::move(vals)}});
    }

    // <-- missing closing } for BrokenContainer
