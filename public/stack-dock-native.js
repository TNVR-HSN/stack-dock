// Load after Stack Dock's existing classic inline script.
(() => {
  'use strict';
  const bridge = window.__TAURI__;
  if (!bridge?.core?.invoke) return;
  const call = (name, payload) => bridge.core.invoke(name, payload);
  const pickPath = mode => call('pick_shortcut_path', { mode });
  const cleanPath = value => value.trim().replace(/^"|"$/g, '');
  // A small, bounded session cache keeps artwork off disk backups and avoids
  // repeatedly asking Windows to classify the same path or decode the same image.
  const imagePath = path => /\.(png|jpe?g|gif|webp|bmp|ico)$/i.test(path);
  const thumbnailEnabled = item => item.imageThumbnail ?? (state.general?.imageThumbnails === true);
  const metadata = new Map(), thumbnails = new Map();
  let activeThumbnails = 0;
  const thumbnailQueue = [];
  function cached(cache, path, limit, get) {
    if (cache.has(path)) return cache.get(path);
    if (cache.size >= limit) cache.delete(cache.keys().next().value);
    const result = Promise.resolve().then(() => get(path)).catch(() => null);
    cache.set(path, result);
    return result;
  }
  const inspectPath = path => cached(metadata, path, 512, async target => {
    const entries = await call('inspect_shortcut_drops', { paths:[target] });
    return entries[0]?.localType || (entries[0]?.kind === 'program' ? 'file' : null);
  });
  const queueThumbnail = target => new Promise(resolve => {
    if (thumbnailQueue.length >= 128) { resolve(null); return; }
    thumbnailQueue.push({target,resolve});
    pumpThumbnails();
  });
  function pumpThumbnails() {
    while (activeThumbnails < 3 && thumbnailQueue.length) {
      const {target,resolve} = thumbnailQueue.shift();
      activeThumbnails++;
      Promise.resolve().then(() => call('read_shortcut_icon', {target,index:0,automatic:false}))
        .then(result => resolve(result?.source ?? null), () => resolve(null))
        .finally(() => { activeThumbnails--; pumpThumbnails(); });
    }
  }
  let watchIcon;
  if (typeof IntersectionObserver === 'function') {
    const waiting = new Map();
    const observer = new IntersectionObserver(entries => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        observer.unobserve(entry.target);
        const load = waiting.get(entry.target);
        waiting.delete(entry.target);
        if (entry.target.isConnected) load?.();
      }
    });
    // Grid redraws replace cells, including offscreen cells that never intersected.
    new MutationObserver(() => {
      for (const node of waiting.keys()) {
        if (!node.isConnected) { waiting.delete(node); observer.unobserve(node); }
      }
    }).observe(document.body,{childList:true,subtree:true});
    watchIcon = (node, load) => queueMicrotask(() => {
      if (!node.isConnected) return;
      waiting.set(node, load); observer.observe(node);
    });
  }
  resolveShortcutVisual = function(node, item, size, radius) {
    if ((item.kind || guess(item.target || '')) !== 'local' || !item.target) return;
    const target = cleanPath(item.target), known = localShortcutType(item);
    const allowed = thumbnailEnabled(item) && imagePath(target);
    if (known === 'folder' || (!allowed && known)) return;
    const load = async () => {
      const kind = known || await inspectPath(target);
      if (!node.isConnected) return;
      const enabled = () => allowed && thumbnailEnabled(item);
      let image = '';
      if (kind !== 'folder' && enabled()) {
        image = await cached(thumbnails, target, 96, async path => {
          const result = await queueThumbnail(path);
          if (!result) thumbnails.delete(path);
          return result;
        });
      }
      if (!node.isConnected) return;
      const key = shortcutIconKey(item, kind);
      const visual = art(key, APP_ICONS, size, enabled() ? image || '' : '', radius);
      node.replaceChildren(...visual.childNodes);
    };
    if (watchIcon) watchIcon(node, load);
    else queueMicrotask(() => { if (node.isConnected) void load(); });
  };
  const basename = path => path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() || path;
  function pathTooltip(input) {
    const sync = () => { input.title = input.value; };
    input.addEventListener('input', sync);
    sync();
    return sync;
  }

  const style = document.createElement('style');
  style.textContent = `
#dockView:focus{outline:none}
dialog.sd-properties{width:min(720px,calc(100% - 24px));--sd-label-width:112px}
.sd-properties:has(.sd-icon-gallery:not([hidden])) .dialogbody{max-height:calc(100dvh - var(--sd-caption-gutter,0px) - 148px);overflow:auto}
.sd-properties .sd-properties-hero{display:flex;align-items:center;justify-content:space-between;gap:14px;min-height:112px;background:#ffffff07;border:1px solid #ffffff13;border-radius:11px;padding:8px 14px;font-size:20px;font-weight:700;color:#e9b0ff;overflow-wrap:anywhere}
.sd-properties .sd-hero-icon{flex:0 0 96px}
.sd-properties .sd-properties-hero .art{border-radius:0;overflow:visible}
.sd-properties .sd-hero-tools{display:flex;align-items:center;justify-content:flex-end;gap:8px;flex:0 0 auto}
.sd-properties .sd-thumb-switch{display:inline-flex;gap:4px;align-items:center;white-space:nowrap;font-size:11px;color:var(--soft);font-weight:500}
.sd-properties .sd-thumb-switch input{accent-color:#d78eeb;margin:0}
.sd-properties .sd-properties-hero>span{min-width:0}
.fields.sd-property-fields{display:grid;grid-template-columns:minmax(0,1fr);gap:8px}
.sd-property-fields>.field{grid-template-columns:var(--sd-label-width,112px) minmax(0,1fr);gap:8px;align-items:center}
.sd-path-controls{display:grid;grid-template-columns:minmax(0,1fr) repeat(var(--sd-buttons,1),31px);align-items:center;gap:5px;min-width:0;position:relative}
.sd-icon-button.quiet{width:31px;height:31px;min-height:0;padding:0;display:grid;place-items:center}
.sd-icon-button svg{width:16px;height:16px}
.sd-icon-detail{grid-column:2;display:flex;align-items:center;gap:8px;color:var(--soft);font-size:11px}
.sd-icon-gallery{padding:10px;display:grid;gap:8px;border:1px solid #dea4f550;border-radius:9px;background:#16101a}
.sd-gallery-path{display:grid;grid-template-columns:minmax(0,1fr) auto;align-items:center;gap:7px}
.sd-gallery-path span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--soft);font-size:12px}
.sd-icon-thumbs{display:grid;grid-template-columns:repeat(auto-fill,minmax(50px,1fr));gap:5px;max-height:180px;min-height:60px;overflow:auto}
.sd-icon-thumbs button{height:50px;display:grid;place-items:center;padding:4px}
.sd-icon-thumbs button[aria-pressed=true]{border-color:#ecbcff;background:#c987dc36}
.sd-icon-thumbs img{width:38px;height:38px;object-fit:contain}
.sd-icon-gallery p{margin:0}
.sd-icon-gallery-actions{display:flex;gap:6px;flex-wrap:wrap}
.sd-icon-gallery-actions .primary{margin-left:auto}
@media(max-width:480px){dialog.sd-properties,.sd-property-fields{--sd-label-width:84px}.sd-properties .sd-properties-hero{font-size:17px}.sd-properties .sd-hero-tools{gap:4px}.sd-properties .sd-hero-icon{width:74px;flex-basis:74px}.sd-properties .sd-hero-icon .art{width:74px;height:74px}}
dialog.sd-panel-region::backdrop{background:transparent}
dialog.sd-panel-region{box-shadow:none}
.sd-modal-active #island,.sd-modal-active .folder-flight,.sd-modal-active #toast{visibility:hidden}
.sd-search-input{width:100%;border:1px solid #ffffff28;background:#140d18;color:#f4ebf8;border-radius:9px;padding:9px 11px}
.sd-search-summary{margin:0}.sd-search-results{display:grid;gap:5px;max-height:min(390px,48dvh);min-height:28px;overflow:auto}
.sd-search-result{display:grid;grid-template-columns:auto minmax(0,1fr) auto;align-items:center;gap:8px;text-align:left;min-width:0}
.sd-search-result.sd-search-selected{background:#ebaeff22;border-color:#dba8ed80}
.sd-search-name,.sd-search-folder{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.sd-search-folder{color:#bdb0c5;font-size:11px;max-width:130px}`;
  document.head.append(style);

  // Keep the fixed Windows host, but expose only the modal while it is open.
  // There is no HWND resize, fullscreen dimmer or screen-sized visible region.
  const originalDialogBuild = dialogBuild;
  let modalObserver = null;
  let modalFrame = null;
  let modalGeneration = 0;

  function stopModalRegion() {
    modalGeneration++;
    dialogMoved=()=>{};
    modalObserver?.disconnect();
    modalObserver = null;
    if (modalFrame !== null) cancelAnimationFrame(modalFrame);
    modalFrame = null;
    dialog.classList.remove('sd-panel-region');
    document.documentElement.classList.remove('sd-modal-active');
  }

  dialog.addEventListener('close', () => {
    // Native close events are queued. A new dialog may already be open.
    if (!dialog.open) stopModalRegion();
  }, true);

  dialogBuild = function (...args) {
    if (!nativeWindow || !fixedHost || !nativeArea) {
      return originalDialogBuild(...args);
    }
    if (dialog.open) return;
    closeMenu(false);
    stopModalRegion();
    const generation = modalGeneration;
    dialog.classList.add('sd-panel-region');
    document.documentElement.classList.add('sd-modal-active');
    try {
      dialogBuildHere(...args);
    } catch (error) {
      stopModalRegion();
      fitNative();
      throw error;
    }

    let applying=false,dirty=false;
    function schedule() {
      dirty=true;
      if (modalFrame!==null||applying) return;
      modalFrame=requestAnimationFrame(()=>{modalFrame=null;fitModalRegion();});
    }
    function fitModalRegion() {
      if (!dialog.open || generation !== modalGeneration || applying) return;
      dirty=false;
      const panel = dialog.getBoundingClientRect();
      if (panel.width <= 0 || panel.height <= 0) return;
      const x = panel.left + nativeHost.x;
      const y = panel.top + nativeHost.y, clipTop = nativeArea.gutter || 0;
      const region = {
        x, y, w: panel.width, h: panel.height,
        ...(y < clipTop || x < 0 ? {clipTop} : {}),
        radius: parseFloat(getComputedStyle(dialog).borderTopLeftRadius) || 0
      };
      if (region.w <= 0 || region.h <= 0) return;
      applying=true;
      void moveHost(region).then(ok => {
        applying=false;
        if (!dialog.open||generation!==modalGeneration) return;
        if (!ok) dialog.close();
        else if(dirty)schedule();
      });
    }
    dialogMoved=schedule;
    fitModalRegion();
    if (typeof ResizeObserver === 'function') {
      modalObserver = new ResizeObserver(schedule);
      modalObserver.observe(dialog);
    }

  };

  // Park keyboard focus on the dock container after Escape, without leaving an icon selected.
  const baseFinishClose = finishClose;
  dockView.tabIndex = -1;
  finishClose = function (refocus) {
    baseFinishClose(false);
    if (refocus) dockView.focus({ preventScroll: true });
  };

  const buttonGlyph = {
    folder: '<path d="M3 7V5a1 1 0 0 1 1-1h5l2 2h7a1 1 0 0 1 1 1v2M3 7v12h15l3-10H7l-2 6"/>',
    reset: '<path d="M4 10a8 8 0 1 1 1 7M4 4v6h6"/>'
  };
  function iconButton(label, glyph, action) {
    const button = B('', 'quiet sd-icon-button', action);
    button.title = label;
    button.setAttribute('aria-label', label);
    button.innerHTML = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'+buttonGlyph[glyph]+'</svg>';
    return button;
  }
  function pathRow(input, buttons) {
    const old = input.closest('label');
    const row = E('div','field');
    const label = E('label','',old.firstElementChild.textContent);
    input.id = 'sd-path-' + uid();
    label.htmlFor = input.id;
    const controls = E('div','sd-path-controls');
    controls.style.setProperty('--sd-buttons', buttons.length);
    controls.append(input,...buttons);row.append(label,controls);
    old.replaceWith(row);
    return {row,controls};
  }

  iconPicker = function (body, source, initial, image, size, round, error, onChange = () => {}, prior = {}, readDefault = null) {
    const defaultKey = () => readDefault?.() || (source === FOLDER_ICONS && Object.hasOwn(source,initial) ? initial : Object.keys(source)[0]);
    let chosenKey = null, key = defaultKey();
    let picture = image || '', path = picture ? prior.iconPath || '' : '';
    let index = Number(prior.iconIndex) || 0, count = Number(prior.iconCount) || (picture ? 1 : 0);
    let mode = picture ? 'custom' : 'default';
    let generation = 0, busy = false, draft = false, timer, targetTimer, renderedKey, renderedPicture;
    const fields = section(body,'Icon');
    fields.classList.add('sd-property-fields');
    const input = field(fields,'Icon path','text',path,{placeholder:'Default icon'});
    const browse = iconButton('Browse icon','folder',() => void chooseSource(null));
    const reset = iconButton('Reset to default icon','reset',resetIcon);
    pathRow(input,[browse,reset]);
    const detail = E('div','sd-icon-detail');
    const change = B('Choose embedded icon…','quiet',()=>void chooseSource(path));
    const status=E('span');status.setAttribute('role','status');
    detail.append(change,status);fields.append(detail);
    const panel=E('section','sd-icon-gallery');
    panel.hidden=true;panel.setAttribute('aria-label','Choose embedded icon');
    const pathBar=E('div','sd-gallery-path');
    const sourceText=E('span');
    const browseAgain=B('Browse…','quiet',()=>void chooseSource(null));
    pathBar.append(sourceText,browseAgain);
    const thumbs=E('div','sd-icon-thumbs');thumbs.setAttribute('role','group');thumbs.setAttribute('aria-label','Available icons');
    const info=E('p','note');info.setAttribute('role','status');
    const actions=E('div','sd-icon-gallery-actions');
    const back=B('Previous','quiet',()=>void page(Math.max(0,offset-24)));
    const next=B('Next','quiet',()=>void page(offset+24));
    const use=B('Use selected icon','primary',()=>void useSelected());
    const cancel=B('Cancel selection','quiet',()=>{stop();closePicker();draw();browse.focus();});
    actions.append(back,next,cancel,use);
    panel.append(E('strong','','Choose an icon'),pathBar,thumbs,info,actions);fields.append(panel);
    const hoverPath = pathTooltip(input), resource = file=>/\.(exe|dll|mun)$/i.test(file);
    const attached = () => dialog.open && error.isConnected;
    let selecting='', selection=null, offset=0, available=0, listTicket=0, listBusy=false, browsing=false;
    function draw(keepDraft = false) {
      if (!keepDraft) input.value = path;
      hoverPath();
      change.hidden=!resource(path);
      change.disabled=busy||!!selecting;
      const message = busy ? 'Reading icon…' : picture && !path ? 'Saved custom icon' : '';
      status.textContent=message;
      detail.hidden = change.hidden && !message;
      if (key!==renderedKey || picture!==renderedPicture) {
        renderedKey=key;renderedPicture=picture;onChange({icon:key,image:picture});
      }
    }
    function stop() {
      clearTimeout(timer);clearTimeout(targetTimer);
      generation++;busy=draft=false;
    }
    function closePicker(){
      listTicket++;listBusy=false;selecting='';selection=null;panel.hidden=true;dialogMoved();
    }
    function updateActions(){
      back.disabled=listBusy||busy||!offset;
      next.disabled=listBusy||busy||offset+24>=available;
      use.disabled=listBusy||busy||selection===null;
    }
    async function page(start){
      if(!selecting)return;
      const ticket=++listTicket,file=selecting;
      offset=start;listBusy=true;thumbs.replaceChildren();info.textContent='Looking for icons…';updateActions();
      try{
        const result=await call('list_shortcut_icons',{target:file,offset:start});
        if(ticket!==listTicket||!attached())return;
        available=result.count;
        if(selection===null||!result.entries.some(entry=>entry.index===selection)){
          selection=result.entries.some(entry=>entry.index===index&&file===path)?index:result.entries[0]?.index??null;
        }
        for(const entry of result.entries){
          const button=B('','quiet',()=>{
            selection=entry.index;
            for(const thumb of thumbs.children)thumb.setAttribute('aria-pressed',String(+thumb.dataset.index===selection));
            updateActions();
          });
          button.dataset.index=entry.index;button.setAttribute('aria-label','Icon '+(entry.index+1));
          button.setAttribute('aria-pressed',String(entry.index===selection));
          const img=E('img');img.src=entry.source;img.alt='';button.append(img);thumbs.append(button);
        }
        info.textContent=!available?'No usable embedded icons. Browse another file or keep the current icon.':
          !result.entries.length?'No readable icons on this page. Try another page or file.':
          (start+1)+'–'+Math.min(available,start+24)+' of '+available+' icon slots. Unreadable icons are skipped.';
      }catch(message){
        if(ticket!==listTicket||!attached())return;
        available=0;selection=null;info.textContent=String(message);
      }finally{
        if(ticket===listTicket){listBusy=false;updateActions();}
      }
    }
    async function chooseSource(file){
      if(browsing)return;
      error.hidden=true;
      if(!file){
        browsing=true;
        try{file=await pickPath('icon');}
        catch(message){if(attached()){error.textContent=String(message);error.hidden=false;}return;}
        finally{browsing=false;}
      }
      if(!file||!attached())return;
      if(resource(file)){
        stop();closePicker();
        selecting=file;sourceText.textContent=file;sourceText.title=file;panel.hidden=false;dialogMoved();
        draw();void page(file===path?Math.floor(index/24)*24:0);
      }else{
        closePicker();await load(file);
      }
    }
    async function load(file, variant = 0) {
      stop();
      const ticket=generation, current=()=>attached()&&ticket===generation;
      busy=true;error.hidden=true;draw(true);updateActions();
      try{
        const result=await call('read_shortcut_icon',{target:file,index:variant,automatic:false});
        if(!current())return false;
        picture=result.source;path=file;index=result.index??variant;count=result.count;mode='custom';
        return true;
      }catch(message){
        if(current()&&selecting){info.textContent=String(message);}
        else if(current()){error.textContent=String(message);error.hidden=false;}
        return false;
      }finally{
        if(current()){busy=false;draw();updateActions();}
      }
    }
    async function useSelected(){
      if(!selecting||selection===null||busy||listBusy)return;
      const file=selecting,variant=selection;
      if(await load(file,variant)&&selecting===file){
        closePicker();draw();browse.focus();
      }
    }
    function resetIcon() { chooseDefault(chosenKey); }
    function chooseDefault(selected) {
      stop();closePicker();error.hidden=true;
      chosenKey=selected;key=chosenKey||defaultKey();picture='';index=count=0;path='';mode='default';
      draw();
    }
    function syncTarget() {
      const nextKey=chosenKey||defaultKey();
      if (key===nextKey) return;
      key=nextKey;
      draw(draft);
    }
    function commitPath() {
      if(!draft)return;
      const value=cleanPath(input.value);
      if(!value){resetIcon();return;}
      if(value===path){stop();draw();return;}
      void chooseSource(value);
    }
    input.addEventListener('input',()=>{
      stop();closePicker();draft=true;error.hidden=true;draw(true);
      timer=setTimeout(()=>{if(attached())commitPath();},320);
    });
    input.addEventListener('change',commitPath);
    input.addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();commitPath();}});
    panel.addEventListener('keydown',e=>{if(e.key==='Escape'){e.preventDefault();e.stopPropagation();cancel.click();}});
    draw();
    queueMicrotask(()=>{if(attached())syncTarget();});
    const read = () => {
      if(draft)commitPath();
      syncTarget();
      if(selecting)throw Error('Use the selected icon or cancel the icon selection before saving.');
      if(busy||browsing)throw Error('The icon is still loading. Try saving again when it’s ready.');
      return {icon:key,image:picture,iconPath:path,iconMode:mode,iconIndex:index,iconCount:count};
    };
    read.syncTarget = (debounce=false) => {
      clearTimeout(targetTimer);
      const update=()=>{
        const previous=renderedKey;
        syncTarget();
        if(previous===renderedKey)onChange({icon:key,image:picture});
      };
      if(debounce)targetTimer=setTimeout(()=>{if(attached())update();},320);
      else update();
    };
    read.chooseDefault = chooseDefault;
    return read;
  };

  dialog.addEventListener('close',()=>{if(!dialog.open)dialog.classList.remove('sd-properties');});
  editShortcut = function (f, where, t) {
    const base = t || { id: uid(), type: 'item', name: '', target: '', kind: 'program', args: '', cwd: '', icon: 'program', image: '' };
    let name, kind, target, args, cwd, readIcon, getTargetChoice, readThumbnailPreference;
    let browsing = false;
    dialogBuild(t ? 'Shortcut properties' : 'Add shortcut here',
      'Row ' + (where.r + 1) + ', column ' + (where.c + 1), 'Save shortcut', (body, error) => {
        dialog.classList.add('sd-properties');
        const hero=E('div','sd-properties-hero'),heroName=E('span','',base.name||'New shortcut'),heroArt=E('span','sd-hero-icon');
        const heroTools=E('div','sd-hero-tools'),thumbLabel=E('label','sd-thumb-switch'),thumb=E('input');
        thumb.type='checkbox';
        thumb.setAttribute('aria-label','Use image as thumbnail for this shortcut');
        thumbLabel.append(thumb,E('span','','Thumbnail'));thumbLabel.hidden=true;
        heroTools.append(thumbLabel,heroArt);hero.append(heroName,heroTools);body.append(hero);
        let thumbnailPref=base.imageThumbnail;
        const p = section(body, 'Shortcut target');
        p.classList.add('sd-property-fields');
        name = field(p, 'Display name', 'text', base.name, { required: true, maxLength: 65 });
        name.addEventListener('input',()=>{heroName.textContent=name.value||'New shortcut';});
        kind = select(p, 'Target type', shortcutTypes, shortcutChoice(base));
        let typeEdited = false;
        const savedChoice = () => !typeEdited && target.value === base.target &&
          (base.kind || guess(base.target)) === 'local' && !localShortcutType(base)
          ? {target:target.value,kind:'local'} : shortcutFromChoice(kind.value,target.value);
        kind.classList.add('sd-target-type');
        target = field(p, 'Target location', 'text', base.target, { required: true });
        args = field(p, 'Arguments', 'text', base.args || '');
        cwd = field(p, 'Start in', 'text', base.cwd || '');
        const updateTargetTooltip = pathTooltip(target),updateFolderTooltip = pathTooltip(cwd);
        async function browse(mode, input) {
          if (browsing) return;
          browsing = true;error.hidden = true;
          try {
            const path = await pickPath(mode);
            if (!path || !input.isConnected || !dialog.open) return;
            input.value = path;
            if (input === target) {
              updateTargetTooltip();
              if (!name.value.trim()) {
                const label=basename(path);
                name.value=(mode==='exe'?label.replace(/\.exe$/i,''):label).slice(0,65);
                name.dispatchEvent(new Event('input',{bubbles:true}));
              }
              readIcon.syncTarget();
            } else updateFolderTooltip();
          } catch (message) {
            if (input.isConnected && dialog.open) {error.textContent = String(message);error.hidden = false;}
          } finally { browsing = false; }
        }
        const targetButton=iconButton('Browse target','folder',()=>{
          void browse(kind.value==='program'?'exe':kind.value,target);
        });
        const targetRow=pathRow(target,[targetButton]);
        const cwdRow=pathRow(cwd,[iconButton('Browse start-in folder','folder',()=>void browse('folder',cwd))]);
        const eligibleForThumbnail=choice=>choice.kind==='local'&&localShortcutType(choice)!=='folder'&&imagePath(cleanPath(choice.target));
        readThumbnailPreference=()=>eligibleForThumbnail(savedChoice())?thumbnailPref:undefined;
        const preview = pick => {
          const choice=savedChoice(), eligible=eligibleForThumbnail(choice), hasCustom=!!pick.image;
          thumbLabel.hidden=!eligible;
          thumb.disabled=hasCustom;
          thumb.checked=thumbnailEnabled({imageThumbnail:thumbnailPref});
          thumbLabel.title=hasCustom?'Reset the custom icon to show an image thumbnail.':
            'Use the image as the icon for this shortcut.';
          heroArt.replaceChildren(shortcutArt({...choice,...pick,imageThumbnail:readThumbnailPreference()},96,'0px'));
        };
        readIcon = iconPicker(body, APP_ICONS, base.icon, customShortcutImage(base), 96, '0px', error,
          preview, base, () => shortcutIconKey(savedChoice()));
        thumb.addEventListener('change',()=>{
          thumbnailPref=thumb.checked;
          readIcon.syncTarget();
        });
        function update() {
          const program = kind.value === 'program', local = kind.value === 'file' || kind.value === 'folder';
          targetButton.hidden = !program && !local;
          targetRow.controls.style.setProperty('--sd-buttons',program||local?1:0);
          args.closest('label').hidden = cwdRow.row.hidden = !program;
          target.placeholder = kind.value === 'url' ? 'https://example.com or steam://…' : 'C:\\Path\\To\\Target';
          if(!program&&!local)targetRow.controls.style.gridTemplateColumns='minmax(0,1fr)';
          else targetRow.controls.style.gridTemplateColumns='';
          readIcon.syncTarget();
        }
        kind.addEventListener('change',()=>{typeEdited=true;update();});
        target.addEventListener('input',()=>{typeEdited=true;readIcon.syncTarget(true);});
        target.addEventListener('change',()=>readIcon.syncTarget());
        getTargetChoice = savedChoice;
        update();
        if(savedChoice().kind==='local'&&!localShortcutType(base)){
          const original = target.value;
          void inspectPath(cleanPath(original)).then(type=>{
            if(!type||typeEdited||!target.isConnected||target.value!==original)return;
            kind.value=type;
            typeEdited=true;
            update();
          });
        }
      }, () => {
        if (browsing) throw Error('Finish choosing the target before saving.');
        const n = name.value.trim(), dest = target.value.trim();
        if (!n || !dest) throw Error('Enter a name and target.');
        if (kind.value === 'url') {
          try { new URL(dest); }
          catch { throw Error('Enter a full URL, including its scheme.'); }
        }
        const pick = readIcon(), program = kind.value === 'program';
        Object.assign(base, { name: n, ...getTargetChoice(), target: dest,
          args: program ? args.value.trim() : '', cwd: program ? cleanPath(cwd.value) : '',
          imageThumbnail:readThumbnailPreference(), ...pick });
        if (!t && !placeEntries(f, where, [base])) throw Error('This grid is full.');
        persist();
        return refresh;
      });
  };

  const backupFormat = 'stack-dock-layout';
  let backupBusy = false;
  function validLayout(data) {
    const fail = message => { throw Error('Invalid Stack Dock layout: ' + message); };
    const object = v => v && typeof v === 'object' && !Array.isArray(v);
    const string = (s, max) => typeof s === 'string' && s.length <= max;
    const int = (n, lo, hi) => Number.isInteger(n) && n >= lo && n <= hi;
    const keys = v => typeof v === 'string' && /^[a-zA-Z0-9_-]{1,128}$/.test(v);
    const picture = v => v === undefined || v === '' ||
      (string(v, 530000) && /^data:image\/(png|jpeg|gif|webp);base64,[A-Za-z0-9+/]+={0,2}$/i.test(v));
    function iconSource(entry) {
      return {
        iconPath:string(entry.iconPath, 32768) ? entry.iconPath : '',
        iconIndex:int(entry.iconIndex, 0, 65535) ? entry.iconIndex : 0,
        iconCount:int(entry.iconCount, 0, 65536) ? entry.iconCount : 0,
        iconMode:['default','target','custom'].includes(entry.iconMode) ? entry.iconMode : undefined
      };
    }
    const seenIds = new Set();
    const edge = ['top', 'bottom', 'left', 'right'];
    const align = ['start', 'center', 'end'];

    function appearanceCopy(input, defaults) {
      if (!object(input)) fail('a dock or grid is missing its appearance settings.');
      const result = { ...defaults };
      const bounds = {
        opacity:[0,100], radius:[0,56], icon:[24,88],
        gap:[0,120], gapX:[0,120], gapY:[0,120], pad:[0,160], padX:[0,160], padY:[0,160],
        hitPadding:[0,48], hoverLabelSize:[9,28], hoverDetailsDelay:[0,5000],
        hoverMode:[0,5], hoverScale:[100,200], hoverLift:[0,48], hoverAngle:[-30,30], hoverDuration:[0,900], hoverCurve:[0,3],
        groundShadowOffset:[-24,48], highlightArea:[0,32], reflectionArea:[0,64], reflectionIntensity:[0,100], groundShadowIntensity:[0,100], borderOpacity:[0,100], borderWidth:[1,5], titleIcon:[16,44],
        motionSpeed:[25,250], hoverOpacity:[0,100], labelFont:[9,18], labelWidth:[64,200],
        launchWidth:[140,680],launchHeight:[42,120],launchHold:[0,8000],launchTravel:[0,1000],
        launchReturn:[0,700],launchIcon:[16,84],launchFont:[11,34],launchRadius:[0,60],launchGap:[0,28],launchPad:[4,28]
      };
      for (const [key, fallback] of Object.entries(defaults)) {
        const val = input[key];
        if (val === undefined) continue;
        const ok = typeof fallback === 'boolean' ? typeof val === 'boolean'
          : typeof fallback === 'string' ? typeof val === 'string' && /^#[\da-f]{6}$/i.test(val)
          : int(val, ...(bounds[key] || [0,100]));
        if (!ok) fail('appearance value "' + key + '" is not valid.');
        result[key] = val;
      }
      for (const key of ['gap','pad']) {
        for (const axis of ['X','Y']) if (input[key+axis] === undefined) result[key+axis] = result[key];
      }
      for (const key of ['iconCorner','highlightCorner']) {
        if(input[key]===undefined)continue;
        if(!int(input[key],0,50))fail('appearance value "'+key+'" is not valid.');
        // Older backups used a raw-mode flag alongside a dormant radius.
        if(key!=='iconCorner'||input.roundIcons!==false)result[key]=input[key];
      }
      return result;
    }

    if (!object(data) || data.format !== backupFormat || data.version !== 1 || !object(data.layout)) {
      fail('choose a version 1 JSON backup exported by Stack Dock.');
    }
    const s = data.layout;
    if (!edge.includes(s.edge) || !align.includes(s.align) ||
        !Array.isArray(s.folders) || !int(s.folders.length, 1, 100)) fail('bad dock position or folder list.');
    const folders = s.folders.map((f,folderIndex) => {
      if (!object(f) || !keys(f.id) || seenIds.has(f.id) || !string(f.name, 100) || !f.name.trim() ||
          !int(f.cols, 1, 12) || !int(f.rows, 1, MAX_ROWS) ||
          !Array.isArray(f.slots) ||
          f.slots.length > f.cols * f.rows || !picture(f.image)) fail('bad folder or grid dimensions.');
      seenIds.add(f.id);
      const cells = new Set(), ids = new Set();
      // Older version-one exports contain synthetic empty cells. Do not restore them.
      const slots = f.slots.filter(t=>t?.type !== 'spacer').map(t => {
        if (!object(t) || !keys(t.id) || ids.has(t.id) || !int(t.r,0,f.rows-1) ||
            !int(t.c,0,f.cols-1) || cells.has(t.r + ':' + t.c)) fail('duplicate or invalid shortcut position.');
        cells.add(t.r + ':' + t.c);
        ids.add(t.id);
        const slot = { id:t.id, r:t.r, c:t.c, type:t.type };
        if (t.type !== 'item' || !string(t.name,1024) || !t.name.trim() || !string(t.target,32768) ||
            !t.target.trim() || !string(t.args ?? '',32768) || !string(t.cwd ?? '',32768) ||
            ![undefined,'program','local','url','uri'].includes(t.kind) || !picture(t.image)) fail('bad shortcut.');
        const localType = ['file','folder'].includes(t.localType) ? t.localType : undefined;
        const imageThumbnail = typeof t.imageThumbnail==='boolean' ? t.imageThumbnail : undefined;
        const custom = customShortcutImage(t);
        return { ...slot, name:t.name, target:t.target, kind:t.kind, localType, imageThumbnail, args:t.args ?? '', cwd:t.cwd ?? '',
          icon:shortcutIconKey({...t,localType}), image:custom,
          ...(custom?iconSource(t):{iconMode:'default'}) };
      });
      return { id:f.id, name:f.name, cols:f.cols, rows:f.rows, slots,
        folderHoverLabels:f.folderHoverLabels!==false,
        style:appearanceCopy(f.style, gsDefaults),
        dockIconShape:['default','rounded','raw'].includes(f.dockIconShape) ? f.dockIconShape : 'default',
        dockIconCorner:int(f.dockIconCorner,0,50) ? f.dockIconCorner : undefined,
        triggerSize:int(f.triggerSize,24,74) ? f.triggerSize : 0,
        icon:Object.hasOwn(FOLDER_ICONS,f.icon) ? f.icon : folderIconKey(folderIndex),
        image:f.image || '', ...iconSource(f) };
    });
    const general={...generalDefaults};
    for(const [key,fallback] of Object.entries(generalDefaults)){
      const value=s.general?.[key];
      if(value===undefined)continue;
      if(typeof fallback==='boolean'?typeof value!=='boolean':!int(value,0,5000))fail('invalid General setting.');
      general[key]=value;
    }
    general.nextFolderColor=int(s.general?.nextFolderColor,0,FOLDER_KEYS.length-1)?s.general.nextFolderColor:folders.length%FOLDER_KEYS.length;
    return { edge:s.edge, align:s.align, general, dock:appearanceCopy(s.dock, dockDefaults), folders };
  }

  const notice = message => {
    if (!dialog.open) dialogBuild('Stack Dock', message, 'OK', () => {}, () => {});
  };
  function replaceLayout(next) {
    // Commit to storage first. A quota error must not overwrite the visible layout.
    try { localStorage.setItem(storageKey, JSON.stringify(next)); }
    catch { throw Error('This layout does not fit in local storage. Your current layout was not changed.'); }
    state = next;
  }

  function confirmLayoutReplacement(next, message) {
    dialogBuild('Replace current layout?', message, 'Replace layout', () => {}, () => {
      replaceLayout(next);
      return refresh;
    });
  }

  async function backupLayout(exporting, report = notice, confirmReplacement = confirmLayoutReplacement) {
    if (backupBusy) return;
    backupBusy = true;
    try {
      const data = exporting ? JSON.stringify({ format:backupFormat, version:1, layout:state }, null, 2) : null;
      const result = await call('stack_dock_layout_backup', { data });
      if (result === null) return;
      if (exporting) {
        report('Layout exported to ' + result);
      } else {
        const next = validLayout(JSON.parse(result));
        const count = next.folders.reduce((n, f) => n + f.slots.filter(t => t.type === 'item').length, 0);
        confirmReplacement(next, 'Import ' + next.folders.length + ' folders and ' + count +
          ' shortcuts? This replaces your current layout. Windows paths are restored as saved.');
      }
    } catch (error) {
      report(error instanceof SyntaxError ? 'This is not a valid JSON backup.' : String(error), true);
    } finally {
      backupBusy = false;
    }
  }

  populateLayoutBackup = function (panel) {
    let pending = null;
    let picking = false;
    const intro = E('p', 'note', 'Export your saved layout as a JSON file, or restore one from a backup.');
    const importButton = B('Import layout…', 'quiet', () => void pick(false));
    const exportButton = B('Export layout…', 'quiet', () => void pick(true));
    const status = E('p', 'note sd-layout-status');
    status.setAttribute('role', 'status');
    status.setAttribute('aria-atomic', 'true');
    status.hidden = true;
    const confirmPanel = E('fieldset', 'section sd-layout-confirm');
    const confirmFields = E('div', 'fields');
    const message = E('p', 'note');
    message.id = 'sd-layout-confirm-message';
    confirmPanel.setAttribute('aria-describedby', message.id);
    const cancel = B('Cancel import', 'quiet', () => {
      pending = null;
      report('');
      sync();
      importButton.focus();
    });
    const replace = B('Replace layout', 'primary', () => {
      if (!pending) return;
      try { replaceLayout(pending); }
      catch (error) { report(error.message, true); return; }
      // The old Settings form refers to folders and preferences that were just replaced.
      dialog.close();
      if (phase === 'open') finishClose(false);
      else refresh();
    });
    confirmFields.append(message, cancel, replace);
    confirmPanel.append(E('legend', '', 'Replace current layout?'), confirmFields);
    panel.append(intro, importButton, exportButton, confirmPanel, status);
    sync();

    function report(text, error = false) {
      status.classList.toggle('err', error);
      status.hidden = !text;
      status.textContent = text;
    }

    function sync() {
      confirmPanel.hidden = !pending;
      importButton.disabled = exportButton.disabled = picking || !!pending;
    }

    async function pick(exporting) {
      if (picking || pending || backupBusy) return;
      const currentForm = form.onsubmit;
      const current = () => dialog.open && form.onsubmit === currentForm;
      picking = true;
      report('');
      sync();
      try {
        await backupLayout(exporting, (text, error) => {
          if (current()) report(text, error);
        }, (next, description) => {
          if (!current()) return;
          pending = next;
          message.textContent = description + ' Unsaved changes in Settings will be discarded.';
        });
      } finally {
        picking = false;
        sync();
        if (current() && panel.isConnected) {
          (pending ? cancel : exporting ? exportButton : importButton).focus({ preventScroll: true });
        }
      }
    }
  };

  function waitForPhase(wanted) {
    return new Promise(resolve => {
      if (phase === wanted) { resolve(true); return; }
      const observer = new MutationObserver(() => {
        if (phase === wanted) done(true);
      });
      const timeout = setTimeout(() => done(false), 4200);
      function done(value) {
        observer.disconnect();
        clearTimeout(timeout);
        resolve(value);
      }
      observer.observe(island, { attributes:true, attributeFilter:['data-phase'] });
    });
  }

  let searchLaunchPending = false;
  async function launchSearchResult(folder, item) {
    if (searchLaunchPending) return;
    searchLaunchPending = true;
    try {
      if (phase === 'open' && active() !== folder) {
        closeStack(false);
        if (!await waitForPhase('closed')) throw Error('Could not close the current grid.');
      }
      if (phase === 'closed') openStack(folder.id);
      if (!await waitForPhase('open') || active() !== folder || !folder.slots.includes(item)) {
        throw Error('Could not open the shortcut’s folder.');
      }
      // Use the installed launcher, including the user's custom URL and local-file handlers.
      await launch(item);
    } catch (error) {
      notice(String(error));
    } finally {
      searchLaunchPending = false;
    }
  }

  function showShortcutSearch() {
    if (dialog.open || searchLaunchPending || !['closed','open'].includes(phase)) return;
    let field, results, summary, matches = [], selected = 0;
    const records = state.folders.flatMap(folder => folder.slots
      .filter(item => item.type === 'item')
      .map(item => ({ folder, item, name:item.name.toLowerCase(),
        searchable:(item.name + ' ' + folder.name + ' ' + item.target).toLowerCase() })));
    const choose = entry => {
      if (!entry) return;
      dialog.close();
      void launchSearchResult(entry.folder, entry.item);
    };
    function draw(reset = true) {
      const query = field.value.trim().toLowerCase();
      const words = query.split(/\s+/).filter(Boolean);
      matches = records.filter(entry => words.every(word => entry.searchable.includes(word)));
      if (query) {
        const score = entry => entry.name === query ? 0 : entry.name.startsWith(query) ? 1 : entry.name.includes(query) ? 2 : 3;
        matches.sort((a,b) => score(a) - score(b));
      }
      const count = matches.length;
      matches = matches.slice(0, 40);
      if (reset) selected = 0;
      summary.textContent = count ? (count > 40 ? 'Showing 40 of ' : '') + count + (count === 1 ? ' shortcut' : ' shortcuts')
        : query ? 'No matching shortcuts.' : 'No shortcuts yet.';
      results.replaceChildren();
      matches.forEach((entry, index) => {
        const button = B('', 'quiet sd-search-result', () => choose(entry));
        button.id = 'sd-shortcut-result-' + index;
        button.tabIndex = index === selected ? 0 : -1;
        button.setAttribute('aria-label', entry.item.name + ' · ' + entry.folder.name);
        button.append(shortcutArt(entry.item,32),
          E('span','sd-search-name',entry.item.name), E('span','sd-search-folder',entry.folder.name));
        results.append(button);
      });
      if (reset) results.scrollTop = 0;
      highlight();
    }
    function highlight() {
      Array.from(results.children).forEach((button, index) => {
        button.classList.toggle('sd-search-selected', index === selected);
        button.tabIndex = index === selected ? 0 : -1;
      });
    }
    function move(delta) {
      if (!matches.length) return;
      selected = (selected + delta + matches.length) % matches.length;
      highlight();
      results.children[selected].scrollIntoView?.({ block:'nearest' });
    }
    dialogBuild('Find shortcut', 'Search by shortcut name, folder, or target. Press Enter to launch.',
      'Launch', body => {
        field = window.document.createElement('input');
        field.className = 'sd-search-input';
        field.type = 'search';
        field.setAttribute('aria-label', 'Search shortcuts');
        field.placeholder = 'Search your dock…';
        field.autocomplete = 'off';
        summary = E('p','note sd-search-summary');
        summary.setAttribute('role','status');
        results = E('div','sd-search-results');
        results.setAttribute('role','group');
        results.setAttribute('aria-label','Search results');
        field.addEventListener('input', () => draw());
        field.addEventListener('keydown', event => {
          if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            move(event.key === 'ArrowDown' ? 1 : -1);
          }
        });
        results.addEventListener('keydown', event => {
          if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            move(event.key === 'ArrowDown' ? 1 : -1);
            results.children[selected]?.focus({ preventScroll:true });
          }
        });
        body.append(field, summary, results);
        draw();
      }, () => {
        const entry = matches[selected];
        if (!entry) throw Error('No matching shortcut to launch.');
        return () => void launchSearchResult(entry.folder, entry.item);
      });
  }

  document.addEventListener('keydown', event => {
    if (event.ctrlKey && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'f') {
      if (dialog.open || !['open','closed'].includes(phase)) return;
      event.preventDefault();
      showShortcutSearch();
    }
  });

  // Add the app submenu after the existing dock menu items, never to the expanded grid.
  const baseDockMenu = dockMenu;
  dockMenu = function (x, y, folder) {
    if (phase !== 'closed') return;
    const baseShow = showMenu;
    showMenu = function (px, py, build, position) {
      return baseShow(px, py, () => {
        build();
        sep();
        mi('Find shortcut…', showShortcutSearch, { hint:'Ctrl+F' });
        const appMenu = mi('Stack Dock', ({x,y}) => {
          showMenu(x,y, () => {
            mt('Stack Dock');
            mi('Back', ({x,y}) => dockMenu(x,y,folder), { hint:'‹' });
            sep();
            mi('Export layout…', () => void backupLayout(true));
            mi('Import layout…', () => void backupLayout(false));
            sep();
            mi('Exit', () => {
              if (phase === 'closed') call('exit_stack_dock', {}).catch(error => notice(String(error)));
            }, { danger:true });
          });
        }, { hint:'›' });
        appMenu.setAttribute('aria-haspopup', 'true');
      }, position);
    };
    try { return baseDockMenu(x, y, folder); }
    finally { showMenu = baseShow; }
  };

  // Explorer supplies full Windows paths through Tauri, not the HTML file name.
  const view = bridge.webview?.getCurrentWebview?.();
  const footerNotice = text => {
    if (dialog.open) return;
    dialogBuild('Stack Dock', text, 'OK', () => {}, () => {});
  };
  const nextCell = f => {
    const map = occupiedMap(f);
    const limit = MAX_ROWS * f.cols;
    for (let i = 0; i < limit; i++) {
      if (!map.has(i)) return { r: Math.floor(i / f.cols), c: i % f.cols };
    }
    return null;
  };

  if (view?.onDragDropEvent) {
    // Prevent the browser's filename-only drop from also adding a duplicate.
    document.addEventListener('drop', event => {
      if (!event.dataTransfer?.files.length) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      mark(null);
    }, true);

    view.onDragDropEvent(async ({ payload }) => {
      if (payload.type === 'leave' || !payload.position || dialog.open) {
        mark(null);
        return;
      }
      const scale = window.devicePixelRatio || nativeArea?.scale || 1;
      const x = payload.position.x / scale, y = payload.position.y / scale;
      const hit = document.elementFromPoint(x, y);
      const grid = phase === 'open' && gsView.contains(hit) ? active() : null;
      const folderId = phase === 'closed' ? hit?.closest('.folder-btn')?.dataset.folder : null;
      const f = grid || state.folders.find(folder => folder.id === folderId);
      const where = f && (grid ? locate(x, y) : nextCell(f));
      if (payload.type !== 'drop') {
        mark(grid ? where : null);
        return;
      }
      mark(null);
      if (!f || !payload.paths?.length) return;
      if (!where) { footerNotice('This folder has no empty slots.'); return; }
      try {
        const paths = await call('inspect_shortcut_drops', { paths: payload.paths });
        if (!state.folders.includes(f)) return;
        const entries = paths.map(item => ({
          ...item, id: uid(), type: 'item', args: '', cwd: '',
          icon: shortcutIconKey(item), image: ''
        }));
        if (!entries.length) { footerNotice('No accessible files or folders were found.'); return; }
        const n = placeEntries(f, where, entries);
        if (!n) { footerNotice('This folder is full.'); return; }
        persist();
        refresh();
      } catch (message) {
        footerNotice(String(message));
      }
    }).catch(error => console.warn('Native drag-and-drop not available:', error));
  }
})();
