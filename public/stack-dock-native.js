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
  style.textContent = '.sd-browse{display:flex;flex-wrap:wrap;gap:5px}.sd-browse button{min-height:29px;padding:3px 8px}.sd-icon-pager{display:flex;align-items:center;gap:6px}.sd-native-icon{display:grid;gap:6px;min-width:0}.sd-native-icon .inline{flex-wrap:wrap}.sd-icon-path{flex:1 1 190px;min-width:0;border:1px solid #ffffff28;background:#140d18;color:#f4ebf8;border-radius:8px;padding:7px 8px}.settings-dialog form{height:min(820px,calc(100dvh - 28px));max-height:none}.settings-dialog .dialogbody,.settings-dialog .settings-body{flex:1 1 0;min-height:0}#dockView:focus{outline:none}';
  style.textContent += `
.native-dock .settings-dialog form{height:min(820px,calc(100dvh - var(--sd-caption-gutter,0px) - 28px))}
.settings-dialog .settings-pane{align-content:start;min-width:0}
.settings-dialog .settings-preview{grid-row:1;position:sticky;top:0;z-index:2;align-self:start;background-color:#211725;box-shadow:0 8px 10px -6px #0f0915}
.settings-dialog .settings-body{scroll-padding-top:160px;overscroll-behavior:contain}
@media(max-height:650px){.settings-dialog .settings-preview{min-height:88px;max-height:112px}.settings-dialog .settings-body{scroll-padding-top:122px}}
dialog.sd-panel-region::backdrop{background:transparent;backdrop-filter:none}
dialog.sd-panel-region{box-shadow:0 5px 16px #07020b8c}
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
    modalObserver?.disconnect();
    modalObserver = null;
    if (modalFrame !== null) cancelAnimationFrame(modalFrame);
    modalFrame = null;
    dialog.classList.remove('sd-panel-region');
  }

  dialog.addEventListener('close', () => {
    // Native close events are queued. A new dialog may already be open.
    if (!dialog.open) stopModalRegion();
  });

  dialogBuild = function (...args) {
    if (!nativeWindow || !fixedHost || !nativeArea) {
      return originalDialogBuild(...args);
    }
    if (dialog.open) return;
    closeMenu(false);
    stopModalRegion();
    const generation = modalGeneration;
    dialog.classList.add('sd-panel-region');
    try {
      dialogBuildHere(...args);
    } catch (error) {
      stopModalRegion();
      fitNative();
      throw error;
    }

    function fitModalRegion() {
      if (!dialog.open || generation !== modalGeneration) return;
      const panel = dialog.getBoundingClientRect();
      if (panel.width <= 0 || panel.height <= 0) return;
      const shadowPad = 12;
      const region = nativeCover({
        x: panel.left + nativeHost.x - shadowPad,
        y: panel.top + nativeHost.y - shadowPad,
        w: panel.width + 2 * shadowPad,
        h: panel.height + 2 * shadowPad
      });
      if (region.w <= 0 || region.h <= 0) return;
      void moveHost(region).then(ok => {
        // Don't strand a modal outside the current clipping region if Windows rejects it.
        if (!ok && dialog.open && generation === modalGeneration) void moveHost(nativeFull());
      });
    }

    fitModalRegion();
    if (typeof ResizeObserver === 'function') {
      modalObserver = new ResizeObserver(() => {
        if (modalFrame !== null || generation !== modalGeneration) return;
        modalFrame = requestAnimationFrame(() => {
          modalFrame = null;
          fitModalRegion();
        });
      });
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

  const originalIconPicker = iconPicker;
  iconPicker = function (body, source, initial, image, size, round, error, onChange = () => {}, programTarget = null, prior = {}) {
    // Wrap the existing gallery so saved icons, built-ins and image uploads continue to work.
    let nativePicture = prior.iconPath && image ? image : '';
    const shell = E('div', 'sd-native-icon');
    body.append(shell);
    const existing = originalIconPicker(shell, source, initial, nativePicture ? '' : image, size, round, error, () => {
      onChange();
    });
    const gallery = shell.querySelector('.gallery');
    const preview = shell.querySelector('.inline > span');
    const custom = E('div', 'inline');
    const pager = E('div', 'sd-icon-pager');
    const status = E('span', 'asset-name');
    const previous = B('Previous', 'quiet');
    const next = B('Next', 'quiet');
    const reset = B('Clear picked icon', 'quiet');
    const browse = B('Browse icon…', 'quiet');
    const pathField = E('input');
    pathField.type = 'text';
    pathField.className = 'sd-icon-path';
    pathField.placeholder = 'Path to an image, ICO, or EXE';
    pathField.setAttribute('aria-label', 'Icon path');
    const loadPath = B('Load path', 'quiet', () => {
      const file = cleanPath(pathField.value);
      if (!file) return;
      choose(file, 0, 'custom');
    });
    pathField.addEventListener('keydown', event => {
      if (event.key === 'Enter') { event.preventDefault(); loadPath.click(); }
    });
    const fromTarget = programTarget && B('Use target EXE icon', 'quiet', () => {
      const file = cleanPath(programTarget());
      if (!/\.exe$/i.test(file)) {
        error.textContent = 'Select a program EXE in Target location first.';
        error.hidden = false;
        return;
      }
      choose(file, 0, 'target');
    });
    let path = nativePicture ? prior.iconPath : '', chosen = Number(prior.iconIndex) || 0;
    let count = nativePicture ? Math.max(chosen + 1, Number(prior.iconCount) || 1) : 0;
    let mode = prior.iconMode || (image ? 'custom' : 'default'), busy = false, generation = 0;
    pathField.value = path;
    const updatePathTooltip = pathTooltip(pathField);

    function updatePager() {
      updatePathTooltip();
      pager.hidden = count <= 1;
      previous.disabled = busy || chosen <= 0;
      next.disabled = busy || chosen + 1 >= count;
      browse.disabled = loadPath.disabled = busy;
      if (fromTarget) fromTarget.disabled = busy;
      reset.hidden = !nativePicture;
      status.textContent = busy ? 'Reading icon…' : nativePicture
        ? basename(path) + (count > 1 ? ' · ' + (chosen + 1) + ' of ' + count : '') : '';
    }

    async function loadIcon(file, index) {
      const result = await call('read_shortcut_icon', { target: file, index });
      const decoded = new Image();
      decoded.src = result.source;
      await decoded.decode();
      const canvas = document.createElement('canvas');
      const factor = 96 / Math.max(decoded.naturalWidth, decoded.naturalHeight);
      canvas.width = Math.max(1, Math.round(decoded.naturalWidth * factor));
      canvas.height = Math.max(1, Math.round(decoded.naturalHeight * factor));
      canvas.getContext('2d').drawImage(decoded, 0, 0, canvas.width, canvas.height);
      return { picture: canvas.toDataURL('image/png'), count: result.count };
    }

    async function choose(fileOrNull, index, nextMode = 'custom', automatic = false) {
      if (busy && !automatic) return;
      const ticket = ++generation;
      busy = true;
      error.hidden = true;
      updatePager();
      try {
        const file = fileOrNull || await pickPath('icon');
        if (!file || !shell.isConnected || !dialog.open || ticket !== generation) return;
        const result = await loadIcon(file, index);
        if (!shell.isConnected || !dialog.open || ticket !== generation) return;
        nativePicture = result.picture;
        path = file;
        pathField.value = file;
        mode = nextMode;
        chosen = index;
        count = result.count;
        gallery.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', 'false'));
        preview.replaceChildren(art(initial, source, size, nativePicture, round));
        onChange();
      } catch (message) {
        if (shell.isConnected && dialog.open && ticket === generation) {
          if (automatic) {
            nativePicture = '';
            path = pathField.value = '';
            count = chosen = 0;
            mode = 'default';
            const pick = existing();
            preview.replaceChildren(art(pick.icon, source, size, pick.image, round));
            gallery.querySelectorAll('button').forEach(button =>
              button.setAttribute('aria-pressed', String(!pick.image && button.dataset.icon === pick.icon)));
            onChange();
          } else {
            error.textContent = String(message);
            error.hidden = false;
          }
        }
      } finally {
        if (ticket === generation) busy = false;
        updatePager();
      }
    }

    function clearPicked(nextMode = 'custom') {
      generation++;
      busy = false;
      nativePicture = '';
      path = pathField.value = '';
      mode = nextMode;
      count = 0;
      chosen = 0;
      updatePager();
    }

    // A built-in pick or browser-file upload overrides a native pick.
    gallery.addEventListener('click', event => { if (event.target.closest('button')) clearPicked(); }, true);
    shell.querySelector('input[type=file]').addEventListener('change', event => {
      const blob = event.target.files[0];
      if (blob && ['image/png','image/jpeg','image/gif','image/webp'].includes(blob.type) && blob.size <= 360000) clearPicked();
    }, true);
    for (const b of shell.querySelectorAll('button')) {
      if (b.textContent === 'Clear') b.addEventListener('click', () => clearPicked('default'), true);
    }
    browse.onclick = () => choose(null, 0);
    previous.onclick = () => choose(path, chosen - 1, mode);
    next.onclick = () => choose(path, chosen + 1, mode);
    reset.onclick = () => {
      clearPicked('default');
      const pick = existing();
      mode = pick.image ? 'custom' : 'default';
      preview.replaceChildren(art(pick.icon, source, size, pick.image, round));
      gallery.querySelectorAll('button').forEach(button =>
        button.setAttribute('aria-pressed', String(!pick.image && button.dataset.icon === pick.icon)));
      onChange();
    };
    pager.append(previous, next);
    custom.append(pathField, loadPath, browse);
    if (fromTarget) custom.append(fromTarget);
    custom.append(pager, status, reset);
    shell.append(custom);
    if (nativePicture) {
      gallery.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', 'false'));
      preview.replaceChildren(art(initial, source, size, nativePicture, round));
    }
    updatePager();
    const read = () => {
      if (busy) throw Error('Finish choosing the icon before saving.');
      return nativePicture
        ? { icon: Object.hasOwn(source, initial) ? initial : Object.keys(source)[0], image: nativePicture,
            iconPath: path, iconIndex: chosen, iconCount: count, iconMode: mode }
        : { ...existing(), iconPath: '', iconIndex: 0, iconCount: 0, iconMode: mode };
    };
    read.useDefaultExe = file => {
      if (mode !== 'custom' && (!nativePicture || path !== file)) return choose(file, 0, 'target', true);
    };
    return read;
  };

  editShortcut = function (f, where, t) {
    const base = t || { id: uid(), type: 'item', name: '', target: '', kind: 'program', args: '', cwd: '', icon: 'web', image: '' };
    let name, kind, target, args, cwd, readIcon;
    let browsing = false;
    dialogBuild(t ? 'Shortcut properties' : 'Add shortcut here',
      'Row ' + (where.r + 1) + ', column ' + (where.c + 1), 'Save shortcut', (body, error) => {
        const p = section(body, 'Shortcut target');
        name = field(p, 'Display name', 'text', base.name, { required: true, maxLength: 65 });
        const savedKind = base.kind || guess(base.target);
        kind = select(p, 'Target type', [
          ['program', 'Program'], ['local', 'File or folder'], ['url', 'Web link']
        ], savedKind === 'uri' ? 'url' : savedKind);
        target = field(p, 'Target location', 'text', base.target, { required: true });
        args = field(p, 'Arguments', 'text', base.args || '');
        cwd = field(p, 'Start in', 'text', base.cwd || '');

        function buttonsFor(input) {
          const row = E('div', 'sd-browse');
          // Keep the button outside the input's label so a click cannot activate both.
          const wrapper = E('div', 'sd-path-row');
          const label = input.closest('label');
          label.before(wrapper);
          wrapper.append(label, row);
          return { row, wrapper };
        }

        const { row: targetButtons } = buttonsFor(target);
        const { row: folderButtons, wrapper: cwdWrapper } = buttonsFor(cwd);
        const updateTargetTooltip = pathTooltip(target);
        const updateFolderTooltip = pathTooltip(cwd);
        async function browse(mode, input) {
          if (browsing) return;
          browsing = true;
          error.hidden = true;
          try {
            const path = await pickPath(mode);
            if (!path || !input.isConnected || !dialog.open) return;
            input.value = path;
            if (input === target) updateTargetTooltip();
            else updateFolderTooltip();
            if (input === target && !name.value.trim()) {
              const label = basename(path);
              name.value = (mode === 'exe' ? label.replace(/\.exe$/i, '') : label).slice(0, 65);
              name.dispatchEvent(new Event('input', { bubbles: true }));
            }
            if (input === target && mode === 'exe') await readIcon.useDefaultExe(path);
          } catch (message) {
            if (input.isConnected && dialog.open) {
              error.textContent = String(message);
              error.hidden = false;
            }
          } finally {
            browsing = false;
          }
        }
        const exe = B('Browse EXE…', 'quiet', () => browse('exe', target));
        const file = B('Browse file…', 'quiet', () => browse('file', target));
        const folder = B('Browse folder…', 'quiet', () => browse('folder', target));
        targetButtons.append(exe, file, folder);
        folderButtons.append(B('Browse folder…', 'quiet', () => browse('folder', cwd)));
        function update() {
          const program = kind.value === 'program', local = kind.value === 'local';
          exe.hidden = !program;
          file.hidden = folder.hidden = !local;
          targetButtons.hidden = !program && !local;
          args.closest('label').hidden = cwdWrapper.hidden = !program;
          target.placeholder = kind.value === 'url' ? 'https://example.com or steam://…' : 'C:\\Path\\To\\Target';
        }
        kind.onchange = update;
        update();
        readIcon = iconPicker(body, APP_ICONS, base.icon, base.image, 38, false, error, () => {},
          () => kind.value === 'program' ? target.value : '', base);
        target.addEventListener('change', () => {
          const file = cleanPath(target.value);
          if (kind.value === 'program' && /\.exe$/i.test(file)) void readIcon.useDefaultExe(file);
        });
      }, () => {
        if (browsing) throw Error('Finish choosing the target before saving.');
        const n = name.value.trim(), dest = target.value.trim(), pick = readIcon();
        if (!n || !dest) throw Error('Enter a name and target.');
        if (kind.value === 'url') {
          try { new URL(dest); }
          catch { throw Error('Enter a full URL, including its scheme.'); }
        }
        const program = kind.value === 'program';
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
        opacity:[0,100], blur:[0,32], radius:[0,56], icon:[24,88],
        gap:[0,32], pad:[0,36], borderOpacity:[0,100], borderWidth:[1,5], titleIcon:[16,44],
        motionSpeed:[25,250], hoverOpacity:[0,100], labelFont:[9,18], labelWidth:[64,200], iconCorner:[0,50]
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
        triggerSize:int(f.triggerSize,24,74) ? f.triggerSize : 0,
        icon:Object.hasOwn(FOLDER_ICONS,f.icon) ? f.icon : Object.keys(FOLDER_ICONS)[0],
        image:f.image || '', ...iconSource(f) };
    });
    return { edge:s.edge, align:s.align, dock:appearanceCopy(s.dock, dockDefaults), folders };
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
        const appMenu = mi('Stack Dock', () => {
          const rect = menu.getBoundingClientRect();
          showMenu(rect.left || px, rect.top || py, () => {
            mt('Stack Dock');
            mi('Back', () => dockMenu(rect.left || px, rect.top || py, folder), { hint:'‹' });
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
