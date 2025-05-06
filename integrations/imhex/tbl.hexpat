import std.io;
import std.core;

fn colour(u64 idx) {
    if (idx % 3 == 0)
        return "ff0000";
    else if (idx % 2 == 0)
        return "00ff00";
    else if (idx % 1 == 0)
        return "0000ff";
    else
        return "f17700";
};

struct Offsets {
    u64 idx = std::core::array_index();
    u16 offset [[color(colour(idx))]];
};

struct Strings {
    u64 idx = std::core::array_index();
    if (idx < std::core::member_count(parent.offsets) - 1)
         u16 size = parent.offsets[idx + 1].offset - parent.offsets[idx].offset;
    else
         u16 size = sizeof($) - parent.offsets[idx].offset;
    char string[size] [[color(idx % 2 ? "005583" : "FFC200")]];
};

struct Data {
    u16 string_count;
    Offsets offsets[string_count];
    Strings strings[string_count];
};

Data data @ 0x00;
