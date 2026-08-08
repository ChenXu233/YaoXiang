import re

path = 'src/std/dict.rs'
s = open(path, encoding='utf-8').read()

# dict.rs arg destructure: "dict.get expects a Dict as first argument",  (no .to_string())
ARG_DICT = re.compile(
    r"let ([a-z_]+) = match args\.first\(\) \{\n"
    r"        Some\(RuntimeValue::Dict\(h\)\) => \*h,\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::type_only\(\n"
    r'                "([a-z_.]+) expects a Dict as first argument"[,)]\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \};",
    re.M)
s, n1 = ARG_DICT.subn(lambda m: f"let {m.group(1)} = expect_dict(args, \"{m.group(2)}\")?;", s)
print("dict arg destructures:", n1)

# heap dict patterns in dict.rs — survey remaining heap.get shapes
HEAP_DICT = re.compile(
    r"match ctx\.heap\.get\(([a-z_]+)\) \{\n"
    r"        Some\(HeapValue::Dict\(map\)\) => map\.clone\(\),\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::runtime_only\(\n"
    r'                "Invalid dict handle"\.to_string\(\),\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \}",
    re.M)
s, n2 = HEAP_DICT.subn(lambda m: f"ctx.heap_dict({m.group(1)})?", s)
print("heap dict clone:", n2)

HEAP_DICT_MUT = re.compile(
    r"match ctx\.heap\.get_mut\(([a-z_]+)\) \{\n"
    r"        Some\(HeapValue::Dict\(map\)\) => map,\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::runtime_only\(\n"
    r'                "Invalid dict handle"\.to_string\(\),\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \}",
    re.M)
s, n3 = HEAP_DICT_MUT.subn(lambda m: f"ctx.heap_dict_mut({m.group(1)})?", s)
print("heap dict mut:", n3)

open(path, 'w', encoding='utf-8').write(s)
print("done")
