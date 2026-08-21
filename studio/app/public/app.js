// Faceless Video Studio — dashboard client (self-hosted, same-origin API)
const $ = (s) => document.querySelector(s);
const $$ = (s) => Array.from(document.querySelectorAll(s));
const API = ''; // same-origin (FastAPI serves both dashboard and API)

const liveDot = $('#provDot'), liveLabel = $('#provLabel');
const emptyState = $('#emptyState'), runningState = $('#runningState'),
      resultState = $('#resultState'), errorState = $('#errorState');
const timeline = $('#timeline'), statusLine = $('#statusLine'),
      imageGrid = $('#imageGrid'), jobTitle = $('#jobTitle');
const form = $('#form'), genBtn = $('#generate');
const STEPS = ['script','audio','image','assemble'];

function setStep(name, state){
  const li = $(`#timeline li[data-step="${name}"]`);
  if(!li) return; li.classList.remove('active','done'); if(state) li.classList.add(state);
}
function resetTimeline(){ STEPS.forEach(s=>{const li=$(`#timeline li[data-step="${s}"]`);li&&li.classList.remove('active','done')}); }
function show(el){ [emptyState,runningState,resultState,errorState].forEach(e=>e.hidden=true); el.hidden=false; }
function busy(s,label){ liveDot.className='dot '+(s||''); if(label) liveLabel.textContent=label; }
function esc(t){ return String(t).replace(/[&<>]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;'}[c])); }

$$('.step-btn').forEach(b=>b.addEventListener('click',()=>{
  const inp=$('#nscenes'); let v=parseInt(inp.value,10)||7; v+=parseInt(b.dataset.dir,10);
  v=Math.max(4,Math.min(10,v)); inp.value=v;
}));

function renderScript(scenes){
  const list=$('#scriptList'); list.innerHTML='';
  scenes.forEach((s,i)=>{
    const li=document.createElement('li');
    li.innerHTML=`<b>Scene ${i+1}.</b> ${esc(s.narration)}`;
    list.appendChild(li);
  });
}
function addImage(idx, url){
  let cell=imageGrid.querySelector(`[data-i="${idx}"]`);
  if(!cell){
    cell=document.createElement('div'); cell.className='cell'; cell.dataset.i=idx;
    const ph=document.createElement('div'); ph.className='ph'; ph.textContent=`Scene ${idx+1}`;
    cell.appendChild(ph); imageGrid.appendChild(cell);
  }
  const img=new Image(); img.onload=()=>{ cell.innerHTML=''; cell.appendChild(img); };
  img.src=`${API}${url}`; img.alt=`Scene ${idx+1}`;
}

async function loadVoices(){
  try{
    const d = await (await fetch(`${API}/api/voices`)).json();
    const sel=$('#voice');
    sel.innerHTML='';
    Object.entries(d.voices).forEach(([k,v])=>{
      const o=document.createElement('option'); o.value=k; o.textContent=v;
      if(k==='nova') o.selected=true; sel.appendChild(o);
    });
    if(d.has_env_key){ busy('live','Server key active'); $('#keyHint').textContent='(server key active — optional)'; }
    else { busy('live','Mock mode — add key'); $('#keyHint').textContent='(add to enable real generation)'; }
  }catch(e){ busy('err','Server offline'); }
}
loadVoices();

async function startRender(){
  const fd=new FormData(form);
  const key=(fd.get('api_key')||'').trim();
  const body={
    topic: fd.get('topic').trim(),
    voice: fd.get('voice'),
    n_scenes: parseInt(fd.get('n_scenes'),10),
    music: fd.get('music')==='on',
  };
  genBtn.disabled=true; genBtn.querySelector('span').textContent='Generating…';
  resetTimeline(); imageGrid.innerHTML=''; $('#scriptList').innerHTML='';
  show(runningState); busy('busy','Rendering');
  jobTitle.textContent='Working…'; statusLine.textContent='Starting…'; setStep('script','active');

  let res;
  try{
    const headers={'Content-Type':'application/json'};
    if(key) headers['x-api-key']=key;
    res=await fetch(`${API}/api/render`,{method:'POST',headers,body:JSON.stringify(body)});
    res=await res.json();
  }catch(e){ return fail('Could not reach the studio server.'); }
  if(!res||!res.job_id) return fail(res?.detail||'Render rejected.');
  openStream(res.job_id);
}

function openStream(jobId){
  const es=new EventSource(`${API}/api/jobs/${jobId}/events`);
  es.addEventListener('script_done',e=>{ const d=JSON.parse(e.data);
    jobTitle.textContent=d.title||'Untitled'; renderScript(d.scenes||[]); setStep('script','done'); setStep('audio','active'); });
  es.addEventListener('audio',e=>{ const d=JSON.parse(e.data); statusLine.textContent=d.status||'Recording…'; setStep('audio','active'); });
  es.addEventListener('image',e=>{ const d=JSON.parse(e.data); statusLine.textContent=d.status||'Creating visual…'; setStep('audio','done'); setStep('image','active'); });
  es.addEventListener('image_done',e=>{ const d=JSON.parse(e.data); addImage(d.index,d.image_url); });
  es.addEventListener('assemble',e=>{ const d=JSON.parse(e.data); statusLine.textContent=d.status||'Assembling…'; setStep('image','done'); setStep('assemble','active'); });
  es.addEventListener('done',e=>{ const d=JSON.parse(e.data);
    const v=$('#resultVideo'); v.src=`${API}${d.video_url}`;
    $('#downloadBtn').href=`${API}${d.video_url}`;
    show(resultState);
  });
  es.addEventListener('complete',()=>{ setStep('assemble','done'); busy('live','Done'); genBtn.disabled=false; genBtn.querySelector('span').textContent='Generate video'; es.close(); });
  es.addEventListener('error',e=>{ try{ const d=JSON.parse(e.data); fail(d.message||'Render failed.'); }catch(_){ fail('Connection lost.'); } es.close(); });
  es.addEventListener('close',()=>{ es.close(); genBtn.disabled=false; });
}

function fail(msg){
  show(errorState); $('#errorMsg').textContent=msg; busy('err','Error');
  genBtn.disabled=false; genBtn.querySelector('span').textContent='Generate video';
}

form.addEventListener('submit',e=>{ e.preventDefault(); startRender(); });
$('#newBtn').addEventListener('click',()=>{ show(emptyState); busy('live',''); });
$('#retryBtn').addEventListener('click',()=>startRender());

(function(){
  const svg=$('#bg'); const w=window.innerWidth,h=window.innerHeight;
  svg.setAttribute('viewBox',`0 0 ${w} ${h}`);
  const blobs=[
    {cx:200,cy:120,r:280,c:'#5fb3a8',o:.10},
    {cx:w-260,cy:340,r:340,c:'#e8a87c',o:.08},
    {cx:w/2,cy:h-120,r:300,c:'#3a5a8c',o:.12},
  ];
  svg.innerHTML=blobs.map((b,i)=>`<circle cx="${b.cx}" cy="${b.cy}" r="${b.r}" fill="${b.c}" opacity="${b.o}"><animate attributeName="cx" values="${b.cx};${b.cx+(i%2?60:-60)};${b.cx}" dur="${18+i*4}s" repeatCount="indefinite"/><animate attributeName="cy" values="${b.cy};${b.cy+(i%2?-40:50)};${b.cy}" dur="${22+i*3}s" repeatCount="indefinite"/></circle>`).join('');
})();
