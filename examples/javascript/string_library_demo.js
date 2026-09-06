// Ported from examples/lumen/string_library_demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function substring(s, from_start, to_end) {
    let index = from_start;
    let out = "";
    while (index < to_end) {
        out = out + s.charAt(index);
        index = index + 1;
    }
    return out;
}

function substring_end(s, from_here) {
    return substring(s, from_here, s.length);
}

function substring_start(s, to_here) {
    return substring(s, 0, to_here);
}

function starts_with(s, prefix) {
    return prefix.length <= s.length && substring(s, 0, prefix.length) === prefix;
}

function ends_with(s, suffix) {
    return suffix.length <= s.length && substring(s, s.length - suffix.length, s.length) === suffix;
}

function repeat_string(s, repetitions) {
    let out = "";
    let i = 0;
    while (i < repetitions) {
        out = out + s;
        i = i + 1;
    }
    return out;
}

function join_strings(arr, separator) {
    let out = "";
    const n = arr.length;
    let i = 0;
    while (i < n) {
        if (i > 0) {
            out = out + separator;
        }
        out = out + arr[i];
        i = i + 1;
    }
    return out;
}

function index_of(s, needle) {
    const n = needle.length;
    let i = 0;
    while (i + n <= s.length) {
        if (substring(s, i, i + n) === needle) {
            return i;
        }
        i = i + 1;
    }
    return -1;
}

function has_substring(s, needle) {
    return index_of(s, needle) >= 0;
}

console.log("=== String Library Examples ===");
console.log("");
const text = "Hello World";
process.stdout.write("Original: ");
console.log(text);
process.stdout.write("substring(text, 0, 5): ");
console.log(substring(text, 0, 5));
process.stdout.write("substring(text, 6, 11): ");
console.log(substring(text, 6, 11));
console.log("");
process.stdout.write("substring_end(text, 6): ");
console.log(substring_end(text, 6));
console.log("");
process.stdout.write("substring_start(text, 5): ");
console.log(substring_start(text, 5));
console.log("");
process.stdout.write("starts_with('Hello World', 'Hello'): ");
console.log(starts_with(text, "Hello"));
process.stdout.write("starts_with('Hello World', 'World'): ");
console.log(starts_with(text, "World"));
console.log("");
process.stdout.write("ends_with('Hello World', 'World'): ");
console.log(ends_with(text, "World"));
process.stdout.write("ends_with('Hello World', 'Hello'): ");
console.log(ends_with(text, "Hello"));
console.log("");
process.stdout.write("repeat_string('Ha', 5): ");
console.log(repeat_string("Ha", 5));
process.stdout.write("repeat_string('-=', 10): ");
console.log(repeat_string("-=", 10));
console.log("");
const fruits = ["apple", "banana", "cherry"];
process.stdout.write("join_strings(['apple', 'banana', 'cherry'], ', '): ");
console.log(join_strings(fruits, ", "));
process.stdout.write("join_strings(['apple', 'banana', 'cherry'], ' | '): ");
console.log(join_strings(fruits, " | "));
console.log("");
const sentence = "The quick brown fox jumps over the lazy dog";
process.stdout.write("index_of('The quick brown fox...', 'fox'): ");
console.log(index_of(sentence, "fox"));
process.stdout.write("index_of('The quick brown fox...', 'cat'): ");
console.log(index_of(sentence, "cat"));
console.log("");
process.stdout.write("has_substring('The quick brown fox...', 'quick'): ");
console.log(has_substring(sentence, "quick"));
process.stdout.write("has_substring('The quick brown fox...', 'slow'): ");
console.log(has_substring(sentence, "slow"));
console.log("");
console.log("=== Practical Example ===");
const name = "Lumen";
const version = "1.0";
const description = "A minimal language";
const separator = repeat_string("-", 40);
console.log(separator);
const info = "Project: " + name;
console.log(info);
const info2 = "Version: " + version;
console.log(info2);
const info3 = "Description: " + description;
console.log(info3);
console.log(separator);
