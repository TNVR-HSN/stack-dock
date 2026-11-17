// Settings and Shortcut Properties appearance. Loaded after the inline app.
const UI_ROLES = Object.freeze({
  window:'Window background', sidebar:'Sidebar and footer', card:'Settings cards',
  preview:'Preview background', previewTint:'Preview tint', input:'Input background',
  selection:'Selected control background', text:'Main text', muted:'Secondary text',
  heading:'Section headings', accent:'Focus and control accent', border:'Borders',
  button:'Primary button background', buttonText:'Primary button text', danger:'Warnings'
});
const UI_PRESETS = Object.freeze(Object.fromEntries([
  ['midnight-plum','Midnight Plum','150f1c 120d18 1b1423 140e1b 873c57 100b16 30203d f2eaf6 ae9db9 d1b0df d29be7 ffffff db9cea 271031 ffabba'],
  ['graphite','Graphite','171a1e 121519 20242a 181d24 3e5163 101318 313945 eef1f4 aeb7c3 c3d2df a9c3d9 c1d3e5 b0c6d8 111923 ffb2b2'],
  ['nordic-frost','Nordic Frost','e9f1f8 dfeaf3 f7fafd e6eef7 9ab9db ffffff ccdeef 1c3043 435b72 25476a 2c628d 3f6384 326c9a ffffff a42d4c'],
  ['ocean-ink','Ocean Ink','0b1b25 0a1620 102633 0c202c 176d83 091820 184153 e2f5fb a2bfca a7e4e9 6bd4e4 9bd6e0 6ccad9 062633 ffafab'],
  ['pine-hollow','Pine Hollow','13201c 0e1a16 1b2b24 15251f 3d6948 0e1713 2b4a39 e9f5eb abc2b0 bae3c0 88d4a0 c0dfc6 8cd4a3 12281c ffb6a8'],
  ['copper-dusk','Copper Dusk','261912 1e140f 302019 241610 945333 1b110c 492c20 faeee4 d1b3a3 edbfa6 e9a37d d8a58c eead86 381c13 ffaea5'],
  ['rosewood','Rosewood','261820 1f131a 33212b 2b1824 803c59 1e1219 502c3e f9eaf1 d5adbe f2b9d2 efa3c5 e0adca eda6c4 391626 ffb3b8'],
  ['indigo-hour','Indigo Hour','151a32 11162c 1e2442 171d39 4c54a3 11162b 303a69 eff1ff b0badb cad1ff a4b4ff b6c0f6 a7b5fb 172044 ffb2c4'],
  ['sandstone','Sandstone','eee6da e5d9c9 fcf6ec f3e8d7 d9a271 fffbf4 e7d1b8 39291f 694e3f 614332 8c5133 88644b 934f32 ffffff 9d3430'],
  ['high-contrast','High Contrast','050505 101010 171717 111111 424242 050505 303030 ffffff e0e0e0 ffde72 ffe267 ffffff ffe267 101010 ffabc1']
].map(([id,name,values])=>{
  const colors=values.split(' ');
  return [id,{name,colors:Object.fromEntries(Object.keys(UI_ROLES).map((key,i)=>[key,'#'+colors[i]]))}];
})));

function normalizeUiTheme(value){
  const preset=typeof value?.preset==='string'&&Object.hasOwn(UI_PRESETS,value.preset)?value.preset:'midnight-plum';
  const defaults=UI_PRESETS[preset].colors;
  const colors=Object.fromEntries(Object.entries(defaults).map(([key,fallback])=>{
    const raw=value?.colors?.[key];
    return [key,typeof raw==='string'&&/^#[0-9a-f]{6}$/i.test(raw)?raw.toLowerCase():fallback];
  }));
  return {preset,custom:value?.custom===true,colors};
}
function uiRgb(color){return [1,3,5].map(i=>parseInt(color.slice(i,i+2),16));}
function uiLuminance(color){
  const [r,g,b]=uiRgb(color).map(n=>n/255).map(n=>n<=.04045?n/12.92:((n+.055)/1.055)**2.4);
  return r*.2126+g*.7152+b*.0722;
}
function uiContrast(first,second){
  const a=uiLuminance(first),b=uiLuminance(second);
  return (Math.max(a,b)+.05)/(Math.min(a,b)+.05);
}
function uiMix(first,second,t){
  const dst=uiRgb(second);
  return '#'+uiRgb(first).map((n,i)=>Math.round(n+(dst[i]-n)*t).toString(16).padStart(2,'0')).join('');
}
function uiLegible(color,background,minimum=4.5){
  if(uiContrast(color,background)>=minimum)return color;
  const endpoint=uiContrast('#ffffff',background)>uiContrast('#000000',background)?'#ffffff':'#000000';
  for(let i=1;i<32;i++){
    const candidate=uiMix(color,endpoint,i/32);
    if(uiContrast(candidate,background)>=minimum)return candidate;
  }
  return endpoint;
}
function uiThemeTokens(value){
  const t=normalizeUiTheme(value),c=t.custom?t.colors:UI_PRESETS[t.preset].colors;
  const accent=uiLegible(c.accent,c.card,3),border=uiLegible(c.border,c.card,3);
  const warning=uiMix(c.card,c.danger,.16),stage=uiMix(c.preview,c.previewTint,.28);
  const switchOff=uiLegible(uiMix(c.card,c.border,.27),c.card,3);
  const desktopScheme=uiLuminance(c.window)>.4?'light':'dark',inputScheme=uiLuminance(c.input)>.4?'light':'dark';
  return {
    win:c.window,rail:c.sidebar,card:c.card,preview:c.preview,stage,input:c.input,selected:c.selection,
    scheme:desktopScheme,'input-scheme':inputScheme,
    text:uiLegible(c.text,c.window),muted:uiLegible(c.muted,c.window),
    'rail-text':uiLegible(c.text,c.sidebar),'rail-muted':uiLegible(c.muted,c.sidebar),
    'card-text':uiLegible(c.text,c.card),'card-muted':uiLegible(c.muted,c.card),
    'stage-text':uiLegible(c.text,stage),'stage-muted':uiLegible(c.muted,stage),
    'preview-text':uiLegible(c.text,c.preview),'preview-muted':uiLegible(c.muted,c.preview),
    'input-text':uiLegible(c.text,c.input),'input-muted':uiLegible(c.muted,c.input),
    'selected-text':uiLegible(c.text,c.selection),heading:uiLegible(c.heading,c.card),
    'preview-heading':uiLegible(c.heading,c.preview),
    accent,'accent-thumb':uiLegible(c.buttonText,accent),'switch-off':switchOff,
    'switch-thumb':uiLegible(c.text,switchOff),
    'rail-focus':uiLegible(c.accent,c.sidebar,3),'input-focus':uiLegible(c.accent,c.input,3),
    'selected-focus':uiLegible(c.accent,c.selection,3),
    line:uiMix(c.card,border,.36),rule:uiMix(c.card,border,.16),
    'input-border':uiLegible(c.border,c.input,3),
    button:c.button,'button-text':uiLegible(c.buttonText,c.button),
    warning,'warning-text':uiLegible(c.danger,warning),'warning-border':uiLegible(c.danger,c.card,3)
  };
}
function applyUiTheme(value){
  const css=document.documentElement.style;
  for(const [name,color] of Object.entries(uiThemeTokens(value)))css.setProperty('--sd-ui-'+name,color);
}
function buildUiThemeSettings(general,onPreview=()=>{}){
  let draft=normalizeUiTheme(state.uiTheme);
  const presets=settingsCard(general,'Theme presets','themes');
  presets.append(E('p','note','Preview a theme here. Your dock and grid colors are independent and will not change.'));
  const gallery=E('div','sd-theme-gallery');
  gallery.setAttribute('role','group');gallery.setAttribute('aria-label','Theme presets');
  presets.append(gallery);
  const editor=settingsCard(general,'Custom theme','themes');
  const custom=check(editor,'Enable custom colors',draft.custom);
  const fields=new Map(),cards=new Map();
  const explain=E('p','note','Text and focus indicators are adjusted when a color is too close to its background.');
  for(const [key,label] of Object.entries(UI_ROLES)){
    const input=field(editor,label,'color',draft.colors[key]);fields.set(key,input);
    const preview=()=>{
      if(!draft.custom)return;
      draft.colors[key]=input.value;update(false);
    };
    input.addEventListener('input',preview);input.addEventListener('change',preview);
  }
  editor.append(explain);
  const bar=E('div','sd-theme-bar');
  bar.hidden=true;
  const message=E('p','note');message.setAttribute('role','status');
  const undo=B('Discard preview','quiet',()=>{
    draft=normalizeUiTheme(state.uiTheme);update(true);
  });
  const save=B('Apply Theme','primary',()=>{
    const next=normalizeUiTheme(draft);
    // Only Apply Theme writes this setting. Avoid persisting any unsaved
    // controls from the rest of the Settings form.
    const savedState={...state,uiTheme:next};
    try{localStorage.setItem(storageKey,JSON.stringify(savedState));}
    catch{message.textContent='Could not save the theme. The current colors are only a preview.';return;}
    state.uiTheme=next;
    update(false);message.textContent='Theme applied.';
  });
  function update(sync){
    if(sync)custom.checked=draft.custom;
    for(const [key,input] of fields){
      input.disabled=!draft.custom;
      input.parentElement.hidden=!draft.custom;
      if(sync)input.value=draft.colors[key];
    }
    for(const [id,button] of cards)button.setAttribute('aria-pressed',String(id===draft.preset&&!draft.custom));
    const previous=normalizeUiTheme(state.uiTheme);
    const dirty=previous.preset!==draft.preset||previous.custom!==draft.custom||
      draft.custom&&Object.keys(draft.colors).some(key=>previous.colors[key]!==draft.colors[key]);
    undo.disabled=save.disabled=!dirty;
    message.textContent=dirty?'Preview only. Apply Theme to keep it.':'No unapplied theme changes.';
    applyUiTheme(draft);onPreview();
  }
  for(const [id,{name,colors}] of Object.entries(UI_PRESETS)){
    const button=B(name,'sd-theme-option',()=>{
      draft={preset:id,custom:false,colors:{...colors}};update(true);
    });
    const swatches=E('span','sd-theme-swatches');swatches.setAttribute('aria-hidden','true');
    for(const key of ['window','card','accent','button']){
      const swatch=E('span');swatch.style.backgroundColor=colors[key];swatches.append(swatch);
    }
    button.prepend(swatches);gallery.append(button);cards.set(id,button);
  }
  custom.addEventListener('change',()=>{draft.custom=custom.checked;update(true);});
  dialog.addEventListener('close',()=>applyUiTheme(state.uiTheme),{once:true});
  bar.append(message,undo,save);
  update(true);
  return bar;
}

// Minimum usable sizes in CSS pixels. A smaller monitor takes precedence.
const SD_DIALOG_MIN=Object.freeze({settings:{width:560,height:365},properties:{width:650,height:365}});
const sdDialogSizes=new Map();
function dialogResizeBounds(kind,work,left=0,top=work.top){
  const width=Math.max(1,work.w-Math.max(0,left)-8);
  const height=Math.max(1,work.h-Math.max(0,top-work.top)-10);
  return {width,height,minW:Math.min(SD_DIALOG_MIN[kind].width,width),minH:Math.min(SD_DIALOG_MIN[kind].height,height)};
}
function setPanelSize(width,height){
  dialog.style.setProperty('--sd-dialog-w',width+'px');
  dialog.style.setProperty('--sd-dialog-h',height+'px');
  dialog.classList.add('sd-dialog-resized');
}
function installDialogResize(kind){
  const previous=sdDialogSizes.get(kind);
  if(previous){
    const b=dialogResizeBounds(kind,dims());
    setPanelSize(clamp(previous.width,b.minW,b.width),clamp(previous.height,b.minH,b.height));
  }
  const grip=B('','sd-dialog-resize');
  grip.setAttribute('aria-label','Resize window with arrow keys');
  grip.title='Drag to resize. Double-click to reset.';
  grip.innerHTML='<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M14 4 4 14m10-5-5 5m5-1-1 1"/></svg>';
  let drag=null;
  function resize(rect,w,h){
    const work=dims(),min=SD_DIALOG_MIN[kind];
    const left=clamp(rect.left,0,Math.max(0,work.w-min.width-8));
    const top=clamp(rect.top,work.top,Math.max(work.top,work.top+work.h-min.height-10));
    const b=dialogResizeBounds(kind,work,left,top);
    const width=clamp(w,b.minW,b.width),height=clamp(h,b.minH,b.height);
    dialog.style.setProperty('--panel-x',left+'px');dialog.style.setProperty('--panel-y',top+'px');
    dialog.classList.add('sd-dialog-placed');
    setPanelSize(width,height);sdDialogSizes.set(kind,{width,height});
    dialogMoved();
  }
  function reset(){
    sdDialogSizes.delete(kind);dialog.classList.remove('sd-dialog-resized','sd-dialog-placed');dialogMoved();
  }
  grip.addEventListener('pointerdown',e=>{
    if(e.button!==0)return;
    drag={id:e.pointerId,x:e.clientX,y:e.clientY,rect:dialog.getBoundingClientRect()};
    grip.setPointerCapture?.(e.pointerId);e.preventDefault();
  });
  grip.addEventListener('pointermove',e=>{
    if(drag?.id!==e.pointerId)return;
    resize(drag.rect,drag.rect.width+e.clientX-drag.x,drag.rect.height+e.clientY-drag.y);
  });
  for(const type of ['pointerup','pointercancel','lostpointercapture']){
    grip.addEventListener(type,e=>{if(drag?.id===e.pointerId)drag=null;});
  }
  grip.addEventListener('dblclick',reset);
  grip.addEventListener('keydown',e=>{
    if(e.key==='Home'){e.preventDefault();reset();return;}
    const step=e.shiftKey?8:24;
    const delta={ArrowLeft:[-step,0],ArrowRight:[step,0],ArrowUp:[0,-step],ArrowDown:[0,step]}[e.key];
    if(!delta)return;
    e.preventDefault();
    const r=dialog.getBoundingClientRect();resize(r,r.width+delta[0],r.height+delta[1]);
  });
  form.append(grip);
}

applyUiTheme(state.uiTheme);
