"""
Real branch-aware Kaspa Script interpreter (subset) for local validation
before mainnet deployment. Implements exactly the opcodes we need, with
correct byte values matching rusty-kaspa's numbering (confirmed via source:
OpIf..OpEndIf span 0x63-0x68 Bitcoin-standard; OpCat/OpXor/OpSha256/etc match
what we've already successfully used on mainnet).
"""
import hashlib

OP_IF, OP_NOTIF, OP_ELSE, OP_ENDIF, OP_VERIFY = 0x63, 0x64, 0x67, 0x68, 0x69
OP_TOALTSTACK, OP_FROMALTSTACK = 0x6b, 0x6c
OP_DUP = 0x76
OP_DROP = 0x75
OP_SWAP = 0x7c
OP_CAT, OP_SUBSTR = 0x7e, 0x7f
OP_AND, OP_OR, OP_XOR, OP_EQUAL, OP_EQUALVERIFY = 0x84, 0x85, 0x86, 0x87, 0x88
OP_ADD, OP_SUB, OP_MUL, OP_DIV, OP_MOD = 0x93, 0x94, 0x95, 0x96, 0x97
OP_NUMEQUAL = 0x9c
OP_LESSTHAN, OP_GREATERTHAN = 0x9f, 0xa0
OP_LESSTHANOREQUAL, OP_GREATERTHANOREQUAL = 0xa1, 0xa2
OP_SHA256 = 0xa8

def cscript_num(b):
    """Decode Kaspa/Bitcoin-style minimally-encoded LE signed integer."""
    if len(b) == 0: return 0
    v = int.from_bytes(b, 'little')
    if b[-1] & 0x80:
        v -= (1 << (8 * len(b)))
    return v

def encode_num(n):
    if n == 0: return b''
    neg = n < 0
    n = abs(n)
    out = bytearray()
    while n:
        out.append(n & 0xff)
        n >>= 8
    if out[-1] & 0x80:
        out.append(0x80 if neg else 0x00)
    elif neg:
        out[-1] |= 0x80
    return bytes(out)

def parse_ops(script: bytes):
    """Parse raw script bytes into a flat opcode/pushdata list with index positions."""
    ops = []
    i = 0
    while i < len(script):
        b = script[i]
        if 1 <= b <= 75:
            ops.append(('push', script[i+1:i+1+b])); i += 1 + b
        elif b == 0x4c:
            n = script[i+1]; ops.append(('push', script[i+2:i+2+n])); i += 2 + n
        elif b == 0x4d:
            n = int.from_bytes(script[i+1:i+3], 'little'); ops.append(('push', script[i+3:i+3+n])); i += 3 + n
        elif b == 0x4e:
            n = int.from_bytes(script[i+1:i+5], 'little'); ops.append(('push', script[i+5:i+5+n])); i += 5 + n
        elif b == 0x00:
            ops.append(('push', b'')); i += 1
        else:
            ops.append(('op', b)); i += 1
    return ops

def execute(script: bytes, initial_stack, hash_op_counter=None, verbose=False):
    ops = parse_ops(script)
    stack = list(initial_stack)
    altstack = []
    # branch state: stack of booleans (whether currently-executing branch is "live")
    cond_stack = []  # each entry: True(executing) / False(skipped) / 'else-done'

    def executing():
        return all(c is True for c in cond_stack)

    i = 0
    while i < len(ops):
        kind, val = ops[i]
        if kind == 'push':
            if executing():
                stack.append(val)
            i += 1; continue

        op = val
        if op == OP_IF or op == OP_NOTIF:
            if executing():
                top = stack.pop()
                truth = (top != b'' and top != b'\x00')
                if op == OP_NOTIF: truth = not truth
                cond_stack.append(truth)
            else:
                cond_stack.append(None)  # placeholder, parent branch not taken -> skip entirely
            i += 1; continue
        if op == OP_ELSE:
            if cond_stack:
                if cond_stack[-1] is not None:
                    cond_stack[-1] = not cond_stack[-1]
            i += 1; continue
        if op == OP_ENDIF:
            if cond_stack: cond_stack.pop()
            i += 1; continue

        if not executing():
            i += 1; continue

        if op == OP_VERIFY:
            v = stack.pop()
            if v == b'' or v == b'\x00':
                raise Exception(f"OP_VERIFY failed at op index {i}")
        elif op == OP_TOALTSTACK:
            altstack.append(stack.pop())
        elif op == OP_FROMALTSTACK:
            stack.append(altstack.pop())
        elif op == OP_DUP:
            stack.append(stack[-1])
        elif op == OP_DROP:
            stack.pop()
        elif op == OP_SWAP:
            b_ = stack.pop(); a_ = stack.pop(); stack.append(b_); stack.append(a_)
        elif op == OP_CAT:
            b_ = stack.pop(); a_ = stack.pop(); stack.append(a_ + b_)
        elif op == OP_SUBSTR:
            end = cscript_num(stack.pop()); start = cscript_num(stack.pop()); d = stack.pop()
            stack.append(d[start:end])
        elif op == OP_XOR:
            b_ = stack.pop(); a_ = stack.pop()
            if len(a_) != len(b_): raise Exception(f'XOR operands must be of equal length at op index {i}')
            stack.append(bytes(x ^ y for x, y in zip(a_, b_)))
        elif op == OP_AND:
            b_ = stack.pop(); a_ = stack.pop()
            if len(a_) != len(b_): raise Exception(f'AND operands must be of equal length at op index {i}')
            stack.append(bytes(x & y for x, y in zip(a_, b_)))
        elif op == OP_OR:
            b_ = stack.pop(); a_ = stack.pop()
            if len(a_) != len(b_): raise Exception(f'OR operands must be of equal length at op index {i}')
            stack.append(bytes(x | y for x, y in zip(a_, b_)))
        elif op == OP_EQUAL:
            b_ = stack.pop(); a_ = stack.pop(); stack.append(b'\x01' if a_ == b_ else b'')
        elif op == OP_EQUALVERIFY:
            b_ = stack.pop(); a_ = stack.pop()
            if a_ != b_: raise Exception(f"OP_EQUALVERIFY failed at op index {i}")
        elif op == OP_NUMEQUAL:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(b'\x01' if a_ == b_ else b'')
        elif op == OP_LESSTHAN:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(b'\x01' if a_ < b_ else b'')
        elif op == OP_GREATERTHAN:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(b'\x01' if a_ > b_ else b'')
        elif op == OP_LESSTHANOREQUAL:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(b'\x01' if a_ <= b_ else b'')
        elif op == OP_GREATERTHANOREQUAL:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(b'\x01' if a_ >= b_ else b'')
        elif op == OP_ADD:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(encode_num(a_ + b_))
        elif op == OP_SUB:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(encode_num(a_ - b_))
        elif op == OP_MUL:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(encode_num(a_ * b_))
        elif op == OP_DIV:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(encode_num(a_ // b_))
        elif op == OP_MOD:
            b_ = cscript_num(stack.pop()); a_ = cscript_num(stack.pop()); stack.append(encode_num(a_ % b_))
        elif op == OP_SHA256:
            v = stack.pop()
            if hash_op_counter is not None: hash_op_counter[0] += 1
            stack.append(hashlib.sha256(v).digest())
        else:
            raise Exception(f"unhandled opcode 0x{op:02x} at index {i}")
        i += 1

    return stack, altstack

print("kvm.py loaded OK — branch-aware interpreter ready")
