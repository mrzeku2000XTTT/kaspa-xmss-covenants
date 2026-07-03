import sys, hashlib
sys.path.insert(0, '/app/xmss-reference/onchain_link')
from xmss_lib import (build_pass1, build_pass2, build_checksum_digits_to_alt, build_chain_verify,
                       build_ltree_combine_v2, build_merkle_walk, KeyMat, F, N_CHAINS, get_digit, N,
                       pd, OP_EQUAL)
from kvm import execute

def Hl(a,b,K,B):
    m = bytes(x^y for x,y in zip(a+b,B)); return hashlib.sha256(b'\x00\x00\x00\x01'+K+m).digest()[:N]
def prf(ps,addr,out_len=N):
    if out_len <= 32:
        return hashlib.sha256(b'\x00\x00\x00\x03'+ps+addr).digest()[:out_len]
    out=b''; counter=0
    while len(out) < out_len:
        out += hashlib.sha256(b'\x00\x00\x00\x03'+ps+addr+bytes([counter])).digest(); counter+=1
    return out[:out_len]
def ab(*w): return b''.join(x.to_bytes(4,'big') for x in w)

def build_merkle_layer(pub_seed, layer_id, target_leaf_idx, height, sec_seed=None):
    n_leaves = 1<<height
    leaves_km = [KeyMat(pub_seed, layer_id, i, sec_seed) for i in range(n_leaves)]
    level_hashes = [km.leaf for km in leaves_km]
    auth_path=[]; idx=target_leaf_idx; cur=level_hashes; lvl=0
    while len(cur) > 1:
        sib = idx ^ 1
        K=prf(pub_seed, ab(layer_id,0,0,3,0,lvl,idx//2,0)); B=prf(pub_seed, ab(layer_id,0,0,3,0,lvl,idx//2,1), out_len=2*N)
        auth_path.append({'auth_node':cur[sib],'is_left':(idx%2==0),'KEY':K,'BM':B})
        nxt=[]
        for i in range(0,len(cur),2):
            k=prf(pub_seed, ab(layer_id,0,0,3,0,lvl,i//2,0)); m=prf(pub_seed, ab(layer_id,0,0,3,0,lvl,i//2,1), out_len=2*N)
            nxt.append(Hl(cur[i],cur[i+1],k,m))
        cur=nxt; idx//=2; lvl+=1
    return leaves_km[target_leaf_idx], auth_path, cur[0]

def build_layer_script(km_leaf, authpath):
    """message must already be on stack top (from scriptSig for layer0, or live root for layer1+)."""
    s = bytearray()
    s += build_pass1() + build_pass2() + build_checksum_digits_to_alt()
    for ci in range(N_CHAINS):
        s += build_chain_verify(km_leaf.chains[ci], km_leaf.chain_ends[ci])
    s += build_ltree_combine_v2(km_leaf.chain_ends, km_leaf.ltree_combines)
    s += build_merkle_walk(authpath)
    return bytes(s)

def sign_layer(km_leaf, message):
    msg_digits=[]
    for b in message: msg_digits.append(b//16); msg_digits.append(b%16)
    csum=sum((15-d) for d in msg_digits); d0=csum//256; rem=csum%256; d1=rem//16; d2=rem%16
    all_digits = msg_digits + [d0,d1,d2]
    witnesses=[]
    for ci in range(N_CHAINS):
        d = get_digit(ci, all_digits); w = km_leaf.sks[ci]
        for st in range(d):
            c = km_leaf.chains[ci][st]; w = F(w, c['KEY'], c['BITMASK'])
        witnesses.append(w)
    return witnesses

H = 4  # small height for fast local proof
pub_seed0 = hashlib.sha256(b'2layer-seed0').digest()[:N]
pub_seed1 = hashlib.sha256(b'2layer-seed1').digest()[:N]
km0, authpath0, root0 = build_merkle_layer(pub_seed0, 0, 5, H)
km1, authpath1, root1 = build_merkle_layer(pub_seed1, 1, 9, H)
MASTER_ROOT = root1

app_message = hashlib.sha256(b'authorize-spend-to-address-XYZ').digest()[:N]
witnesses0 = sign_layer(km0, app_message)
witnesses1 = sign_layer(km1, root0)   # layer1 signs layer0's root -- REAL linkage, computed here off-chain for witness-gen, verified ON-CHAIN below

script = build_layer_script(km0, authpath0) + build_layer_script(km1, authpath1)
script += pd(MASTER_ROOT) + bytes([OP_EQUAL])

initial_stack = list(reversed(witnesses1)) + list(reversed(witnesses0)) + [app_message]
stack, alt = execute(script, initial_stack)
print("Full 2-layer script size:", len(script), "bytes")
print("Final stack:", [x.hex() if isinstance(x,bytes) else x for x in stack])
print("VERIFICATION RESULT:", "PASS" if stack and stack[-1]==b'\x01' else "FAIL")

# forgery: tamper with layer0 witness (should break the WHOLE chain since root0 feeds layer1)
bad_w0 = list(witnesses0); bad_w0[5] = bytes([1]*24)
initial_bad = list(reversed(witnesses1)) + list(reversed(bad_w0)) + [app_message]
try:
    stack2, alt2 = execute(script, initial_bad)
    print("FORGERY (bad layer0 witness):", "REJECTED" if not stack2 or stack2[-1]!=b'\x01' else "!!ACCEPTED")
except Exception as e:
    print("FORGERY (bad layer0 witness) raised exception (rejected):", str(e)[:60])
