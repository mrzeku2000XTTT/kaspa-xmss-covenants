import sys, hashlib
sys.path.insert(0, '/app/xmss-reference/onchain_link')
from kvm import execute, encode_num, cscript_num

def pd(b):
    if len(b)==0: return bytes([0x00])
    if len(b)<=75: return bytes([len(b)])+b
    if len(b)<=255: return bytes([0x4c,len(b)])+b
    lb=len(b).to_bytes(2,'little'); return bytes([0x4d])+lb+b

OP_IF,OP_ELSE,OP_ENDIF=0x63,0x67,0x68
OP_TOALT,OP_FROMALT,OP_DUP,OP_SWAP,OP_DROP=0x6b,0x6c,0x76,0x7c,0x75
OP_CAT,OP_SUBSTR,OP_XOR,OP_EQUAL,OP_EQUALVERIFY=0x7e,0x7f,0x86,0x87,0x88
OP_ADD,OP_SUB,OP_DIV,OP_MOD,OP_SHA256=0x93,0x94,0x96,0x97,0xa8
OP_LESSTHANOREQUAL=0xa1
N=24; W=16; STEPS=15; N_CHAINS=51

def F(inp,KEY,BM):
    m=bytes(a^b for a,b in zip(inp,BM)); return hashlib.sha256(b'\x00\x00\x00\x00'+KEY+m).digest()[:N]
def Hl(a,b,KEY,BM):
    m=bytes(x^y for x,y in zip(a+b,BM)); return hashlib.sha256(b'\x00\x00\x00\x01'+KEY+m).digest()[:N]
def prf(ps,addr,out_len=N):
    if out_len <= 32:
        return hashlib.sha256(b'\x00\x00\x00\x03'+ps+addr).digest()[:out_len]
    out = b''
    counter = 0
    while len(out) < out_len:
        out += hashlib.sha256(b'\x00\x00\x00\x03'+ps+addr+bytes([counter])).digest()
        counter += 1
    return out[:out_len]
def ab(*w): return b''.join(x.to_bytes(4,'big') for x in w)

def f_ops(K,B):
    s=bytearray(); s+=pd(B)+bytes([OP_XOR]); s+=pd(K)+bytes([OP_SWAP])+bytes([OP_CAT])
    s+=pd(b'\x00\x00\x00\x00')+bytes([OP_SWAP])+bytes([OP_CAT]); s+=bytes([OP_SHA256])
    s+=pd(encode_num(0))+pd(encode_num(N))+bytes([OP_SUBSTR]); return bytes(s)

def build_pass1():
    s=bytearray(); s+=bytes([OP_TOALT])
    for i in range(N):
        s+=bytes([OP_FROMALT])
        if i<N-1: s+=bytes([OP_DUP])
        s+=pd(encode_num(i))+pd(encode_num(i+1))+bytes([OP_SUBSTR])
        s+=pd(b'\x00')+bytes([OP_CAT])
        if i<N-1: s+=bytes([OP_SWAP]); s+=bytes([OP_TOALT])
        s+=bytes([OP_DUP]); s+=pd(encode_num(W))+bytes([OP_DIV])
        s+=bytes([OP_SWAP]); s+=pd(encode_num(W))+bytes([OP_MOD])
    return bytes(s)

def build_pass2(n=48):
    s=bytearray(); s+=pd(encode_num(0))
    for _ in range(n):
        s+=bytes([OP_SWAP]); s+=bytes([OP_DUP]); s+=bytes([OP_TOALT])
        s+=pd(encode_num(W-1))+bytes([OP_SWAP])+bytes([OP_SUB]); s+=bytes([OP_ADD])
    return bytes(s)

def build_checksum_digits_to_alt():
    s=bytearray()
    s+=bytes([OP_DUP])+pd(encode_num(W*W))+bytes([OP_DIV])
    s+=bytes([OP_SWAP])+pd(encode_num(W*W))+bytes([OP_MOD])
    s+=bytes([OP_DUP])+pd(encode_num(W))+bytes([OP_DIV])
    s+=bytes([OP_SWAP])+pd(encode_num(W))+bytes([OP_MOD])
    s+=bytes([OP_TOALT])*3
    return bytes(s)

def build_chain_verify(steps, chain_end):
    s=bytearray()
    s+=bytes([OP_FROMALT])       # stack: [witness, digit]  (value=witness, digit on top -- already correct shape)
    for step in range(STEPS):
        c=steps[step]
        s+=bytes([OP_DUP])
        s+=pd(encode_num(step))+bytes([OP_LESSTHANOREQUAL])
        s+=bytes([OP_IF])
        s+=bytes([OP_SWAP])
        s+=f_ops(c['KEY'],c['BITMASK'])
        s+=bytes([OP_SWAP])
        s+=bytes([OP_ELSE])
        s+=bytes([OP_ENDIF])
    s+=bytes([OP_DROP])
    s+=pd(chain_end)+bytes([OP_EQUALVERIFY])
    return bytes(s)

class KeyMat:
    def __init__(self, pub_seed, layer_id, leaf_idx=0, sec_seed=None):
        # sec_seed is the REAL secret — must be random, user-generated, never shared.
        # Falls back to a fixed test string only if not supplied (legacy/demo mode).
        if sec_seed is None:
            sec_seed = b'LEGACY-TEST-SEED-NOT-SECURE'
        self.chains=[]; self.chain_ends=[]; self.sks=[]
        for ci in range(N_CHAINS):
            sk=hashlib.sha256(sec_seed + f'-L{layer_id}-leaf{leaf_idx}-c{ci}'.encode()).digest()[:N]
            self.sks.append(sk); steps=[]; cur=sk
            for st in range(STEPS):
                K=prf(pub_seed, ab(layer_id,0,0,0,leaf_idx*1000+ci,0,st,0))
                B=prf(pub_seed, ab(layer_id,0,0,0,leaf_idx*1000+ci,0,st,1))
                nxt=F(cur,K,B); steps.append({'KEY':K,'BITMASK':B}); cur=nxt
            self.chains.append(steps); self.chain_ends.append(cur)
        # L-tree combine
        self.ltree_combines=[]
        level=list(self.chain_ends); li=0
        while len(level)>1:
            nxt=[]; combines=[]; i=0
            while i+1<len(level):
                K=prf(pub_seed, ab(layer_id,0,0,2,leaf_idx,li,i//2,0))
                B=prf(pub_seed, ab(layer_id,0,0,2,leaf_idx,li,i//2,1), out_len=2*N)
                out=Hl(level[i],level[i+1],K,B); combines.append({'KEY':K,'BM':B}); nxt.append(out); i+=2
            odd=(len(level)%2==1)
            if odd: nxt.append(level[-1])
            self.ltree_combines.append({'combines':combines,'odd':odd}); level=nxt; li+=1
        self.leaf=level[0]

def get_digit(ci, all_digits):
    if ci < 3: return [all_digits[48],all_digits[49],all_digits[50]][ci]  # d0,d1,d2
    j = ci-3
    return all_digits[j]  # message digit j (0..47) in natural order


def build_ltree_combine(chain_ends, ltree_combines):
    """
    Pushes all len(chain_ends) known constants fresh (reverse order, so index0 ends up on top),
    then combines pairwise per ltree_combines structure, reducing to 1 leaf value.
    """
    s = bytearray()
    for v in reversed(chain_ends):
        s += pd(v)
    # stack: [v_last,...,v1,v0] i.e v0 on TOP
    n = len(chain_ends)
    for level in ltree_combines:
        combines = level['combines']; odd = level['odd']
        # process each pair: pop a(top),b(next) -> Hl(a,b) -> push to altstack
        for c in combines:
            s += bytes([OP_TOALT])   # wait need a,b both popped THEN combined, not individually stashed
        # (placeholder loop above is wrong; real implementation below)
    return None

def build_ltree_combine_v2(chain_ends, ltree_combines):
    s = bytearray()
    for v in reversed(chain_ends):
        s += pd(v)
    for level in ltree_combines:
        combines = level['combines']; odd = level['odd']
        for c in combines:
            # stack top2 = [a(2nd from top? or top?), b] -- need a=top(first popped)=index i, b=next=index i+1
            # our push order put index0 on top, so popping order matches index ascending: first pop=a(index i), second pop=b(index i+1)
            s += f_combine_ops(c['KEY'], c['BM'])   # consumes top 2, produces combined value on main stack top
            s += bytes([OP_TOALT])   # park result out of the way while more pairs at this level get processed
        if odd:
            s += bytes([OP_TOALT])   # park the leftover unpaired item too
        # now main stack is empty for this level's inputs; move all results back from altstack (reverses order again)
        cnt = len(combines) + (1 if odd else 0)
        for _ in range(cnt):
            s += bytes([OP_FROMALT])
    return bytes(s)

def f_combine_ops(KEY, BM):
    """Hl(a,b,KEY,BM). Stack entering: [...,b,a] (a=TOP=first popped, corresponds to lower index v_i)."""
    s = bytearray()
    s += bytes([OP_SWAP])                       # fix order: -> [...,a,b] so CAT gives a||b
    s += bytes([OP_CAT])
    s += pd(BM) + bytes([OP_XOR])
    s += pd(KEY) + bytes([OP_SWAP]) + bytes([OP_CAT])
    s += pd(b'\x00\x00\x00\x01') + bytes([OP_SWAP]) + bytes([OP_CAT])
    s += bytes([OP_SHA256])
    s += pd(encode_num(0)) + pd(encode_num(N)) + bytes([OP_SUBSTR])
    return bytes(s)

def build_merkle_walk(authpath):
    """Leaf value on stack top. Walks auth path to root using Hl combine."""
    s = bytearray()
    for lvl in authpath:
        s += pd(lvl['auth_node'])   # stack: [...,leaf,auth_node] (auth_node TOP)
        if lvl['is_left']:
            s += bytes([OP_SWAP])   # -> [...,auth_node,leaf] matches f_combine_ops's [b,a] shape (a=leaf TOP)
        s += f_combine_ops(lvl['KEY'], lvl['BM'])
    return bytes(s)
