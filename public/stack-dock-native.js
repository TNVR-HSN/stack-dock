// Load after Stack Dock's existing classic inline script.
(() => {
  'use strict';
  const bridge = window.__TAURI__;
  if (!bridge?.core?.invoke) return;
  const call = (name, payload) => bridge.core.invoke(name, payload);
  const pickPath = mode => call('pick_shortcut_path', { mode });
  const cleanPath = value => value.trim().replace(/^"|"$/g, '');
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
.sd-properties .sd-properties-hero{display:flex;align-items:center;justify-content:space-between;gap:14px;min-height:112px;background:#ffffff07;border:1px solid #ffffff13;border-radius:11px;padding:8px 14px;font-size:20px;font-weight:700;color:#e9b0ff;overflow-wrap:anywhere}
.sd-properties .sd-hero-icon{flex:0 0 96px}
.sd-properties .sd-properties-hero .art{border-radius:0;overflow:visible}
.fields.sd-property-fields{display:grid;grid-template-columns:minmax(0,1fr);gap:8px}
.sd-property-fields>.field{grid-template-columns:var(--sd-label-width,112px) minmax(0,1fr);gap:8px;align-items:center}
.sd-path-controls{display:grid;grid-template-columns:minmax(0,1fr) repeat(var(--sd-buttons,1),31px);align-items:center;gap:5px;min-width:0;position:relative}
.sd-icon-button.quiet{width:31px;height:31px;min-height:0;padding:0;display:grid;place-items:center}
.sd-icon-button svg{width:16px;height:16px}
.sd-local-picker{position:absolute;right:0;top:100%;z-index:3;display:grid;width:120px;padding:4px;gap:3px;background:#241a29;border:1px solid #d9a5e350;border-radius:8px}
.sd-local-picker button{text-align:left;min-height:28px}
.sd-icon-detail{grid-column:2;display:flex;align-items:center;gap:8px;color:var(--soft);font-size:11px}
.sd-icon-detail select{width:auto}
@media(max-width:480px){dialog.sd-properties,.sd-property-fields{--sd-label-width:84px}.sd-properties .sd-properties-hero{font-size:17px}}
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
      const y = Math.max(nativeArea.gutter || 0, panel.top + nativeHost.y);
      const region = {
        x, y, w: panel.width, h: panel.top + panel.height + nativeHost.y - y,
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

  iconPicker = function (body, source, initial, image, size, round, error, onChange = () => {}, programTarget = null, prior = {}) {
    const defaultKey = source === APP_ICONS ? 'web' : 'folder-emerald';
    let key = Object.hasOwn(source,initial) ? initial : defaultKey;
    let picture = image || '', path = prior.iconPath || '';
    let index = Number(prior.iconIndex) || 0, count = Number(prior.iconCount) || (picture ? 1 : 0);
    let mode = prior.iconMode || (picture ? 'custom' : 'default');
    let generation = 0, busy = false, draft = false, timer, targetTimer, renderedKey, renderedPicture;
    const fields = section(body,'Icon');
    fields.classList.add('sd-property-fields');
    const input = field(fields,'Icon path','text',path,{placeholder:'Default icon'});
    const browse = iconButton('Browse icon','folder',() => void load(null,0,'custom'));
    const reset = iconButton('Reset icon to automatic','reset',() => resetIcon(true));
    pathRow(input,[browse,reset]);
    const detail = E('div','sd-icon-detail');
    const variants = E('select');
    variants.setAttribute('aria-label','Embedded icon');
    const status = E('span');
    status.setAttribute('role','status');
    detail.append(variants,status);
    fields.append(detail);
    const hoverPath = pathTooltip(input);
    const attached = () => dialog.open && error.isConnected;
    const currentTarget = () => {
      const target = cleanPath(programTarget?.() || '');
      return /\.exe$/i.test(target) ? target : '';
    };
    function draw(keepDraft = false) {
      if (!keepDraft) input.value = path;
      hoverPath();
      if (variants.options.length !== count && count > 1) {
        variants.replaceChildren(...Array.from({length:count},(_,i)=>{
          const option=E('option','','Icon '+(i+1));option.value=i;return option;
        }));
      } else if (count <= 1 && variants.options.length) variants.replaceChildren();
      variants.value = String(index);
      variants.hidden = count <= 1;
      variants.disabled = busy || draft;
      const message = busy ? 'Reading icon…' : mode==='target' ? 'From target EXE' : picture && !path ? 'Saved custom icon' : '';
      status.textContent = message;
      detail.hidden = variants.hidden && !message;
      if (key!==renderedKey || picture!==renderedPicture) {
        renderedKey=key;renderedPicture=picture;onChange({icon:key,image:picture});
      }
    }
    function stop() {
      clearTimeout(timer);clearTimeout(targetTimer);
      generation++;busy=draft=false;
    }
    async function load(file, variant = 0, nextMode = 'custom') {
      stop();
      const ticket = generation;
      const current = () => attached() && ticket===generation;
      busy = true;error.hidden = true;
      draw(file !== null);
      try {
        file = file || await pickPath('icon');
        if (!file || !current()) return;
        const result = await call('read_shortcut_icon',{target:file,index:variant});
        if (!current()) return;
        picture=result.source;path=file;index=variant;count=result.count;mode=nextMode;
      } catch (message) {
        if (current() && nextMode !== 'target') {
          error.textContent=String(message);error.hidden=false;
        }
      } finally {
        if(current()){busy=false;draw();}
      }
    }
    function resetIcon(explicit = false) {
      stop();error.hidden=true;
      if(explicit)key=defaultKey;
      picture='';index=count=0;
      path=currentTarget();mode=path?'target':'default';
      draw();
      if(path)void load(path,0,'target');
    }
    function syncTarget() {
      if (mode==='custom' || draft) return;
      const target=currentTarget();
      if (target===path) return;
      resetIcon();
    }
    function commitPath() {
      if(!draft)return;
      const value=cleanPath(input.value);
      if(!value){resetIcon(true);return;}
      if(value===path){stop();draw();return;}
      void load(value,0,'custom');
    }
    input.addEventListener('input',()=>{
      stop();draft=true;error.hidden=true;draw(true);
      timer=setTimeout(()=>{if(attached())commitPath();},320);
    });
    input.addEventListener('change',commitPath);
    input.addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();commitPath();}});
    variants.addEventListener('change',()=>void load(path,+variants.value,mode));
    draw();
    queueMicrotask(()=>{if(attached())syncTarget();});
    const read = () => {
      if(draft)commitPath();
      syncTarget();
      if(busy)throw Error('The icon is still loading. Try saving again when it’s ready.');
      return {icon:key,image:picture,iconPath:path,iconMode:mode,iconIndex:index,iconCount:count};
    };
    read.syncTarget = (debounce=false) => {
      clearTimeout(targetTimer);
      if(debounce)targetTimer=setTimeout(()=>{if(attached())syncTarget();},320);
      else syncTarget();
    };
    return read;
  };

  dialog.addEventListener('close',()=>{if(!dialog.open)dialog.classList.remove('sd-properties');});
  editShortcut = function (f, where, t) {
    const base = t || { id: uid(), type: 'item', name: '', target: '', kind: 'program', args: '', cwd: '', icon: 'web', image: '' };
    let name, kind, target, args, cwd, readIcon;
    let browsing = false;
    dialogBuild(t ? 'Shortcut properties' : 'Add shortcut here',
      'Row ' + (where.r + 1) + ', column ' + (where.c + 1), 'Save shortcut', (body, error) => {
        dialog.classList.add('sd-properties');
        const hero=E('div','sd-properties-hero'),heroName=E('span','',base.name||'New shortcut'),heroArt=E('span','sd-hero-icon');
        hero.append(heroName,heroArt);body.append(hero);
        const p = section(body, 'Shortcut target');
        p.classList.add('sd-property-fields');
        name = field(p, 'Display name', 'text', base.name, { required: true, maxLength: 65 });
        name.addEventListener('input',()=>{heroName.textContent=name.value||'New shortcut';});
        const savedKind = base.kind || guess(base.target);
        kind = select(p, 'Target type', [
          ['program', 'Program'], ['local', 'File or folder'], ['url', 'Web link']
        ], savedKind === 'uri' ? 'url' : savedKind);
        target = field(p, 'Target location', 'text', base.target, { required: true });
        args = field(p, 'Arguments', 'text', base.args || '');
        cwd = field(p, 'Start in', 'text', base.cwd || '');
        const updateTargetTooltip = pathTooltip(target),updateFolderTooltip = pathTooltip(cwd);
        async function browse(mode, input) {
          choices.hidden=true;
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
        const choices = E('div','sd-local-picker');choices.hidden=true;
        choices.append(B('Choose file','quiet',()=>void browse('file',target)),B('Choose folder','quiet',()=>void browse('folder',target)));
        const targetButton=iconButton('Browse target','folder',()=>{
          if(kind.value==='local'){choices.hidden=!choices.hidden;if(!choices.hidden)choices.firstElementChild.focus();}
          else void browse('exe',target);
        });
        const targetRow=pathRow(target,[targetButton]);
        targetRow.controls.append(choices);
        targetRow.controls.addEventListener('focusout',event=>{
          if(!event.relatedTarget || !targetRow.controls.contains(event.relatedTarget))choices.hidden=true;
        });
        targetRow.controls.addEventListener('keydown',event=>{
          if(event.key==='Escape'&&!choices.hidden){event.preventDefault();event.stopPropagation();choices.hidden=true;targetButton.focus();}
        });
        const cwdRow=pathRow(cwd,[iconButton('Browse start-in folder','folder',()=>void browse('folder',cwd))]);
        readIcon = iconPicker(body, APP_ICONS, base.icon, base.image, 96, '0px', error,
          pick=>heroArt.replaceChildren(art(pick.icon,APP_ICONS,96,pick.image,'0px')),
          () => kind.value === 'program' || kind.value === 'local' ? target.value : '', base);
        function update() {
          const program = kind.value === 'program', local = kind.value === 'local';
          choices.hidden=true;
          targetButton.hidden = !program && !local;
          targetRow.controls.style.setProperty('--sd-buttons',program||local?1:0);
          args.closest('label').hidden = cwdRow.row.hidden = !program;
          target.placeholder = kind.value === 'url' ? 'https://example.com or steam://…' : 'C:\\Path\\To\\Target';
          if(!program&&!local)targetRow.controls.style.gridTemplateColumns='minmax(0,1fr)';
          else targetRow.controls.style.gridTemplateColumns='';
          readIcon.syncTarget();
        }
        kind.addEventListener('change',update);
        target.addEventListener('input',()=>readIcon.syncTarget(true));
        target.addEventListener('change',()=>readIcon.syncTarget());
        update();
      }, () => {
        if (browsing) throw Error('Finish choosing the target before saving.');
        const n = name.value.trim(), dest = target.value.trim();
        if (!n || !dest) throw Error('Enter a name and target.');
        if (kind.value === 'url') {
          try { new URL(dest); }
          catch { throw Error('Enter a full URL, including its scheme.'); }
        }
        const pick = readIcon(), program = kind.value === 'program';
        Object.assign(base, { name: n, target: dest, kind: kind.value,
          args: program ? args.value.trim() : '', cwd: program ? cleanPath(cwd.value) : '', ...pick });
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
    const folders = s.folders.map(f => {
      if (!object(f) || !keys(f.id) || seenIds.has(f.id) || !string(f.name, 100) || !f.name.trim() ||
          !int(f.cols, 1, 12) || !int(f.rows, 1, MAX_ROWS) ||
          !['auto', 'rows'].includes(f.mode) || !Array.isArray(f.slots) ||
          f.slots.length > f.cols * f.rows || !picture(f.image)) fail('bad folder or grid dimensions.');
      seenIds.add(f.id);
      const cells = new Set(), ids = new Set();
      const slots = f.slots.map(t => {
        if (!object(t) || !keys(t.id) || ids.has(t.id) || !int(t.r,0,f.rows-1) ||
            !int(t.c,0,f.cols-1) || cells.has(t.r + ':' + t.c)) fail('duplicate or invalid shortcut position.');
        cells.add(t.r + ':' + t.c);
        ids.add(t.id);
        const slot = { id:t.id, r:t.r, c:t.c, type:t.type };
        if (t.type === 'spacer') return slot;
        if (t.type !== 'item' || !string(t.name,1024) || !t.name.trim() || !string(t.target,32768) ||
            !t.target.trim() || !string(t.args ?? '',32768) || !string(t.cwd ?? '',32768) ||
            ![undefined,'program','local','url','uri'].includes(t.kind) || !picture(t.image)) fail('bad shortcut.');
        return { ...slot, name:t.name, target:t.target, kind:t.kind, args:t.args ?? '', cwd:t.cwd ?? '',
          icon:Object.hasOwn(APP_ICONS,t.icon) ? t.icon : 'web', image:t.image || '',
          ...iconSource(t) };
      });
      return { id:f.id, name:f.name, cols:f.cols, rows:f.rows, mode:f.mode, slots,
        style:appearanceCopy(f.style, gsDefaults),
        dockIconShape:['default','rounded','raw'].includes(f.dockIconShape) ? f.dockIconShape : 'default',
        dockIconCorner:int(f.dockIconCorner,0,50) ? f.dockIconCorner : undefined,
        triggerSize:int(f.triggerSize,24,74) ? f.triggerSize : 0,
        icon:Object.hasOwn(FOLDER_ICONS,f.icon) ? f.icon : Object.keys(FOLDER_ICONS)[0],
        image:f.image || '', ...iconSource(f) };
    });
    const general={...generalDefaults};
    for(const [key,fallback] of Object.entries(generalDefaults)){
      const value=s.general?.[key];
      if(value===undefined)continue;
      if(typeof fallback==='boolean'?typeof value!=='boolean':!int(value,500,5000))fail('invalid General setting.');
      general[key]=value;
    }
    return { edge:s.edge, align:s.align, general, dock:appearanceCopy(s.dock, dockDefaults), folders };
  }

  const notice = message => {
    if (!dialog.open) dialogBuild('Stack Dock', message, 'OK', () => {}, () => {});
  };
  async function backupLayout(exporting) {
    if (backupBusy) return;
    backupBusy = true;
    try {
      const data = exporting ? JSON.stringify({ format:backupFormat, version:1, layout:state }, null, 2) : null;
      const result = await call('stack_dock_layout_backup', { data });
      if (result === null) return;
      if (exporting) {
        notice('Layout exported to ' + result);
      } else {
        const next = validLayout(JSON.parse(result));
        const count = next.folders.reduce((n, f) => n + f.slots.filter(t => t.type === 'item').length, 0);
        dialogBuild('Replace current layout?',
          'Import ' + next.folders.length + ' folders and ' + count + ' shortcuts? This replaces your current layout. Windows paths are restored as saved.',
          'Replace layout', () => {}, () => {
            // Commit to storage first. A quota error must not overwrite the visible layout.
            try { localStorage.setItem(storageKey, JSON.stringify(next)); }
            catch { throw Error('This layout does not fit in local storage. Your current layout was not changed.'); }
            state = next;
            return refresh;
          });
      }
    } catch (error) {
      notice(error instanceof SyntaxError ? 'This is not a valid JSON backup.' : String(error));
    } finally {
      backupBusy = false;
    }
  }

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
        button.append(art(entry.item.icon, APP_ICONS, 32, entry.item.image),
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
          icon: item.kind === 'program' ? 'terminal' : 'web', image: ''
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
