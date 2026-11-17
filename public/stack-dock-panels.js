// Settings and Shortcut Properties appearance. Loaded after the inline app.
const UI_ROLES = Object.freeze({
  window:'Window background', sidebar:'Sidebar and footer', card:'Settings cards',
  preview:'Preview background', previewTint:'Preview tint', input:'Input background',
  selection:'Selected control background', text:'Main text', muted:'Secondary text',
  heading:'Section headings', accent:'Focus and control accent', border:'Borders',
  button:'Primary button background', buttonText:'Primary button text', danger:'Warnings'
});
// Both variants are tuned separately to keep the preset's hue identity.
const UI_PRESETS = Object.freeze(Object.fromEntries([
  ['midnight-plum','Midnight Plum','dark',
   '150f1c 120d18 1b1423 140e1b 873c57 100b16 30203d f2eaf6 ae9db9 d1b0df d29be7 e9cefb db9cea 271031 ffabba',
   'f1ebf5 ece3f1 f8f3f9 f0e6f3 d6a0c2 fcf8fd e7d7ee 32233b 69556f 754885 87509f 634774 b780cb 23132c 9a3657'],
  ['graphite','Graphite','dark',
   '171a1e 121519 20242a 181d24 3e5163 101318 313945 eef1f4 aeb7c3 c3d2df a9c3d9 c1d3e5 b0c6d8 111923 ffb2b2',
   'e9edf0 e2e7ec f3f5f7 e8edf1 a5b7c8 f7f9fa dce4ec 28343f 4b5866 40576a 4d6578 4d6072 7794ad 131d26 9b393f'],
  ['nordic-frost','Nordic Frost','light',
   '121b29 101723 1b2635 162133 4c6b96 0f1825 253b53 e5effa a5bbd2 b8d4ef 8fbae5 b4d0ed a0c5ea 112239 ffadb9',
   'eaf1f8 e2eaf3 f5f8fb e7eff7 a7c3e2 fbfdff dae7f4 24384c 485f75 345675 416f96 4b6883 73a4ce 102637 9e3553'],
  ['ocean-ink','Ocean Ink','dark',
   '101c25 0d1720 172631 13222d 246c81 0d1923 213c4c e2f3f9 a6bec8 abd9e3 78c7d7 a7d6df 7bc5d4 103038 ffafab',
   'e7f0f2 ddebed f3f8f9 e1edf0 96cbd4 f8fbfc d2e8ec 183640 42616c 285966 3d7485 476670 75adba 0d292f 9d3b44'],
  ['pine-hollow','Pine Hollow','dark',
   '13201c 0e1a16 1b2b24 15251f 3d6948 0e1713 2b4a39 e9f5eb abc2b0 bae3c0 88d4a0 c0dfc6 8cd4a3 12281c ffb6a8',
   'e9f0e9 e0ebe1 f3f7f2 e6efe6 9bc3a3 f8fbf7 d8e8db 253c2d 476451 365c41 4b7956 4c6c54 86b294 132c1c 963e46'],
  ['copper-dusk','Copper Dusk','dark',
   '241913 1d140f 2d211b 221915 8b543b 1a120e 483026 faeee4 d1b3a3 edbfa6 e5aa89 d8a58c e4ae8f 381c13 ffaea5',
   'f3eae2 eee0d5 f9f2ed f3e6dc d9ae96 fbf7f3 eedbcd 3e2b21 6c5245 754d37 9b6145 785743 d29b7d 311a12 9c3d38'],
  ['rosewood','Rosewood','dark',
   '241920 1e141b 2e212a 281923 793c58 1c1219 4b2d3e f9eaf1 d5adbe f2b9d2 e7a6c3 e0adca e5a6c1 391626 ffb3b8',
   'f4eaf0 efdee9 faf3f7 f4e7ef d5a2bd fdf8fb eed8e6 41293a 6d4e64 7c4b69 9e597f 7d526d cf95b6 39172d 9c3958'],
  ['indigo-hour','Indigo Hour','dark',
   '161b2e 12172a 20263c 171e35 4c54a3 111629 2e395e eff1ff b0badb cad1ff a4b4ff b6c0f6 a7b5fb 172044 ffb2c4',
   'ebedf8 e2e5f4 f5f6fc e8eafa a5a6db fbfbff dce2f5 2b3151 535b7f 464d83 5e68aa 535986 9a9ed1 1e224a 9f395c'],
  ['sandstone','Sandstone','light',
   '231d17 1c1712 2d251e 251d16 886040 19140f 443628 f7ede2 d1bda8 ebd0b2 e0b98d d6bda6 d6ae83 352618 ffb1a0',
   'efe8de e8dfd3 f8f1e9 f1e8da d3aa87 fbf7f1 ebd9c4 39291f 694e3f 614332 8c5b3f 88644b c99d78 311f13 9d3430'],
  ['high-contrast','High Contrast','dark',
   '0c0c0d 09090a 181819 111112 5d4b24 050505 333020 ffffff e0e0e0 ffde72 ffe267 f7e6b6 ffe267 101010 ffabc1',
   'f4f2e9 e8e6dc fcfbf5 f1efe4 d7bc71 fffffb f1e1ac 17150c 383023 57430a 6c5100 4c432e f5cf63 231b04 8f1938']
].map(([id,name,initialMode,dark,light])=>{
  const palette=source=>{
    const colors=source.split(' ');
    return Object.fromEntries(Object.keys(UI_ROLES).map((key,i)=>[key,'#'+colors[i]]));
  };
  return [id,{name,initialMode,variants:{dark:palette(dark),light:palette(light)}}];
})));

function normalizeUiTheme(value){
  const preset=typeof value?.preset==='string'&&Object.hasOwn(UI_PRESETS,value.preset)?value.preset:'midnight-plum';
  const choice=UI_PRESETS[preset];
  const mode=['dark','light'].includes(value?.mode)?value.mode:choice.initialMode;
  const defaults=choice.variants[mode];
  const colors=Object.fromEntries(Object.entries(defaults).map(([key,fallback])=>{
    const raw=value?.colors?.[key];
    return [key,typeof raw==='string'&&/^#[0-9a-f]{6}$/i.test(raw)?raw.toLowerCase():fallback];
  }));
  return {preset,mode,custom:value?.custom===true,colors};
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
  const t=normalizeUiTheme(value),c=t.custom?t.colors:UI_PRESETS[t.preset].variants[t.mode];
  const accent=uiLegible(c.accent,c.card,3);
  const warning=uiMix(c.card,c.danger,.16),stage=uiMix(c.preview,c.previewTint,.26);
  const stageBase=uiMix(c.preview,c.window,.35),stageHalo=uiMix(c.preview,c.accent,.13);
  const emphasized=!t.custom&&t.preset==='high-contrast';
  // Borders reveal hierarchy, not focus. Reserve strong contrast for the
  // text, control states and actual keyboard focus ring.
  const switchOff=uiMix(c.card,c.border,emphasized?.29:.15);
  const switchOn=uiMix(c.selection,c.accent,emphasized?.83:.54);
  const desktopScheme=uiLuminance(c.window)>.4?'light':'dark',inputScheme=uiLuminance(c.input)>.4?'light':'dark';
  return {
    win:c.window,rail:c.sidebar,card:c.card,preview:c.preview,stage,input:c.input,selected:c.selection,
    'stage-base':stageBase,'stage-halo':stageHalo,
    scheme:desktopScheme,'input-scheme':inputScheme,
    text:uiLegible(c.text,c.window),muted:uiLegible(c.muted,c.window),
    'rail-text':uiLegible(c.text,c.sidebar),'rail-muted':uiLegible(c.muted,c.sidebar),
    'card-text':uiLegible(c.text,c.card),'card-muted':uiLegible(c.muted,c.card),
    'stage-text':uiLegible(c.text,stage),'stage-muted':uiLegible(c.muted,stage),
    'preview-text':uiLegible(c.text,c.preview),'preview-muted':uiLegible(c.muted,c.preview),
    'input-text':uiLegible(c.text,c.input),'input-muted':uiLegible(c.muted,c.input),
    'selected-text':uiLegible(c.text,c.selection),heading:uiLegible(c.heading,c.card),
    'preview-heading':uiLegible(c.heading,c.preview),
    accent,'switch-off':switchOff,'switch-on':switchOn,
    'switch-thumb':uiLegible(c.text,switchOff),'switch-on-thumb':uiLegible(c.text,switchOn),
    'switch-on-border':uiMix(switchOn,c.accent,.38),
    'rail-focus':uiLegible(c.accent,c.sidebar,3),'input-focus':uiLegible(c.accent,c.input,3),
    'selected-border':uiMix(c.selection,c.accent,emphasized?.44:.19),
    'selected-rail-border':uiMix(c.selection,c.border,emphasized?.18:.06),
    line:uiMix(c.card,c.border,emphasized?.18:.075),
    rule:uiMix(c.card,c.border,emphasized?.09:.036),
    'input-border':uiMix(c.input,c.border,emphasized?.22:.13),
    scrollbar:uiMix(c.window,c.accent,emphasized?.65:.40),
    button:c.button,'button-sheen':uiMix(c.button,'#ffffff',.12),
    'button-text':uiLegible(c.buttonText,c.button),
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
  presets.append(E('p','note','Choose a preset and switch between Dark and Light without changing its color family. Dock and grid colors stay independent.'));
  const modeBar=E('div','sd-theme-modes');
  const modeControl=E('div','sd-theme-mode-control');
  modeControl.setAttribute('role','group');modeControl.setAttribute('aria-label','Preset color mode');
  const modeButtons=new Map();
  for(const [mode,label] of [['dark','Dark'],['light','Light']]){
    const button=B(label,'',()=>{
      if(draft.custom||draft.mode===mode)return;
      draft={...draft,mode,colors:{...UI_PRESETS[draft.preset].variants[mode]}};
      update(true);
    });
    modeControl.append(button);modeButtons.set(mode,button);
  }
  modeBar.append(E('span','','Mode'),modeControl);
  presets.append(modeBar);
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
    for(const [mode,button] of modeButtons){
      button.disabled=draft.custom;
      button.setAttribute('aria-pressed',String(mode===draft.mode));
    }
    for(const [id,{button,swatches}] of cards){
      button.setAttribute('aria-pressed',String(id===draft.preset&&!draft.custom));
      const colors=UI_PRESETS[id].variants[draft.mode];
      for(const [key,node] of swatches)node.style.backgroundColor=colors[key];
    }
    const previous=normalizeUiTheme(state.uiTheme);
    const dirty=previous.preset!==draft.preset||previous.mode!==draft.mode||previous.custom!==draft.custom||
      draft.custom&&Object.keys(draft.colors).some(key=>previous.colors[key]!==draft.colors[key]);
    undo.disabled=save.disabled=!dirty;
    message.textContent=dirty?'Preview only. Apply Theme to keep it.':'No unapplied theme changes.';
    applyUiTheme(draft);onPreview();
  }
  for(const [id,{name}] of Object.entries(UI_PRESETS)){
    const button=B(name,'sd-theme-option',()=>{
      draft={preset:id,mode:draft.mode,custom:false,colors:{...UI_PRESETS[id].variants[draft.mode]}};
      update(true);
    });
    const strip=E('span','sd-theme-swatches');strip.setAttribute('aria-hidden','true');
    const swatches=new Map();
    for(const key of ['window','card','accent','button']){
      const swatch=E('span');strip.append(swatch);swatches.set(key,swatch);
    }
    button.prepend(strip);gallery.append(button);cards.set(id,{button,swatches});
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
    dialog.classList.remove('sd-settings-initial-position');
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
    sdDialogSizes.delete(kind);dialog.classList.remove('sd-dialog-resized','sd-dialog-placed','sd-settings-initial-position');
    if(liveSettingsOpen())placeLiveSettings();
    dialogMoved();
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
