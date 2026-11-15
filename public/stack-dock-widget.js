// Load once, after stack-dock-native.js. Works with the fixed-host Stack Dock.
(() => {
  'use strict';
  const invoke = window.__TAURI__?.core?.invoke;
  const style = document.createElement('style');
  style.textContent = `
#dockView .folder-btn,#gsView .tile{position:relative}
#dockView .folder-btn:hover,#gsView .tile:hover{background:none}
#dockView .folder-btn:hover>.art,#gsView .tile:hover>.art{box-shadow:0 0 0 4px rgb(255 255 255 / var(--sd-hover,.14)), inset 0 0 0 100px rgb(255 255 255 / calc(var(--sd-hover,.14) / 2))}
#gsView.sd-labels .tile{display:grid;grid-template-columns:minmax(0,1fr);grid-template-rows:var(--sd-icon-size) var(--sd-reflect) var(--sd-label-space);row-gap:2px;align-content:start;justify-items:center;padding:5px 3px}
#gsView.sd-labels .tile>.art{grid-row:1}
#gsView.sd-labels .tile>.reflect{grid-row:2}
#gsView.sd-labels .tile>.caption{grid-row:3;align-self:start;font-size:var(--sd-label-font);line-height:1.2;height:2.4em;max-height:2.4em;overflow:hidden;overflow-wrap:anywhere;word-break:break-word;text-overflow:ellipsis}
.sd-edge-grow{position:fixed;z-index:51;pointer-events:none;background:#d99bff80;border:1px solid #ead1ff;border-radius:5px}
dialog.sd-properties{width:min(670px,calc(100vw - 24px))}
.sd-properties .sd-properties-hero{display:flex;align-items:center;justify-content:space-between;min-height:86px;background:#ffffff07;border:1px solid #ffffff13;border-radius:11px;padding:10px 14px;font-size:20px;font-weight:700;color:#e9b0ff;overflow-wrap:anywhere}
.sd-properties .sd-properties-hero img,.sd-properties .sd-properties-hero svg{width:60px;height:60px;object-fit:contain}
.sd-properties .sd-properties-rows{display:grid;grid-template-columns:1fr;gap:8px}
.sd-properties .sd-properties-rows>label.field{grid-template-columns:112px minmax(0,1fr);align-items:center}
.sd-properties .sd-properties-rows>.sd-path-row{display:grid;grid-template-columns:minmax(0,1fr);gap:5px;min-width:0}
.sd-properties .sd-path-row>label.field{display:grid;grid-template-columns:minmax(0,1fr);gap:4px;min-width:0}
.sd-properties .sd-native-icon .sd-icon-path{flex:0 0 100%;width:100%}
.sd-properties .sd-native-icon{border-top:1px solid #ffffff18;padding-top:8px}
.sd-properties .sd-native-icon .gallery{max-height:84px}
@media(max-width:450px){.sd-properties .sd-properties-rows>label.field{grid-template-columns:85px minmax(0,1fr)}}
`;
  document.head.append(style);

  const numeric = (v, lo, hi, fallback) => Number.isFinite(v) ? Math.max(lo, Math.min(hi, v)) : fallback;
  Object.assign(dockDefaults, {motionSpeed:100, hoverOpacity:14});
  Object.assign(gsDefaults, {labelFont:11, labelWidth:76, iconCorner:25, hoverOpacity:14});
  // New appearance keys are preserved by the matching layout backup script.
  const baseAppearance = appearance;
  appearance = function (body, s, isGrid) {
    const read = baseAppearance(body, s, isGrid);
    const panel = section(body, isGrid ? 'Tile styling and label layout' : 'Motion and hover');
    const slider = (key, name, min, max, unit, fallback) =>
      field(panel, name, 'range', numeric(s[key],min,max,fallback), {min,max,datasetUnit:unit});
    const hover = slider('hoverOpacity','Hover highlight',0,100,'%',14);
    const round = select(panel,'Icon corners',[['rounded','Rounded'],['raw','Original shape']],s.roundIcons===false?'raw':'rounded');
    if (isGrid) {
      const font = slider('labelFont','Label size',9,18,' px',11);
      const width = slider('labelWidth','Minimum label width',64,200,' px',76);
      const corner = slider('iconCorner','Icon roundness',0,50,'%',25);
      const sync = () => {corner.disabled=round.value==='raw';};
      round.onchange=sync;sync();
      return () => ({...read(),labelFont:+font.value,labelWidth:+width.value,iconCorner:+corner.value,hoverOpacity:+hover.value,roundIcons:round.value==='rounded'});
    }
    const speed = slider('motionSpeed','Animation speed',25,250,'%',100);
    return () => ({...read(),motionSpeed:+speed.value,hoverOpacity:+hover.value,roundIcons:round.value==='rounded'});
  };

  const baseMetrics = metrics;
  let measuring;
  metrics = function (folder) {
    const m = baseMetrics(folder), s = folder.style;
    if (!s.labels) return m;
    const font = numeric(s.labelFont,9,18,11);
    const minWidth = Math.max(m.cw,numeric(s.labelWidth,64,200,76));
    try { measuring ||= document.createElement('canvas').getContext('2d'); } catch {}
    const widthOf = measuring
      ? label => { measuring.font = font + 'px "Segoe UI",system-ui,sans-serif'; return measuring.measureText(label).width; }
      : label => Array.from(label).length * font * .57;
    function fits(name, width) {
      const chunks = name.trim().split(/\s+/);
      let row = '', used = 1;
      for (const chunk of chunks) {
        if (widthOf(chunk) > width) {
          for (const c of chunk) {
            if (widthOf(row + c) > width) { used++; row = ''; }
            row += c;
          }
        } else if (!row || widthOf(row + ' ' + chunk) <= width) {
          row += (row ? ' ' : '') + chunk;
        } else { used++; row = chunk; }
        if (used > 2) return false;
      }
      return true;
    }
    let w = minWidth;
    // Every tile uses the same width and every tile reserves exactly two label lines.
    for (const item of folder.slots) {
      if (item.type !== 'item') continue;
      while (w < 200 && !fits(item.name,w-8)) w = Math.min(200,w+4);
      if (w === 200) break;
    }
    return { ...m,cw:w,ch:s.icon+(s.reflections?13:0)+Math.ceil(font*2.4)+16 };
  };
  const baseRenderGrid = renderGrid;
  renderGrid = function (folder,...args) {
    const s = folder.style;
    gsView.classList.toggle('sd-labels',!!s.labels);
    gsView.style.setProperty('--sd-icon-size',s.icon+'px');
    gsView.style.setProperty('--sd-reflect',(s.reflections?10:0)+'px');
    const font = numeric(s.labelFont,9,18,11);
    gsView.style.setProperty('--sd-label-font',font+'px');
    gsView.style.setProperty('--sd-label-space',Math.ceil(font*2.4)+'px');
    gsView.style.setProperty('--sd-hover',numeric(s.hoverOpacity,0,100,14)/100);
    return baseRenderGrid(folder,...args);
  };
  const baseRenderDock = renderDock;
  renderDock = function (...args) {
    dockView.style.setProperty('--sd-hover',numeric(state.dock.hoverOpacity,0,100,14)/100);
    return baseRenderDock(...args);
  };

  const baseMorph = morph, baseFlight = startFlight;
  morph = function (...args) {
    const factor = 100/numeric(state.dock.motionSpeed,25,250,100);
    if (factor === 1) return baseMorph(...args);
    const undo = [];
    for (const node of [island,dockGlass,gsGlass,dockView,gsView]) {
      if (typeof node.animate !== 'function') continue;
      const own = Object.hasOwn(node,'animate'), original = node.animate;
      node.animate = function (frames,options) {
        return original.call(this,frames,{...options,duration:options.duration*factor,delay:(options.delay||0)*factor});
      };
      undo.push(() => {if(own) node.animate=original; else delete node.animate;});
    }
    startFlight = function (f,a,b,c,duration,...rest) { return baseFlight(f,a,b,c,duration*factor,...rest); };
    try { return baseMorph(...args); }
    finally { startFlight=baseFlight; undo.forEach(fn=>fn()); }
  };

  const oldProperties = editShortcut;
  editShortcut = function (...args) {
    const oldDialog = dialogBuild;
    dialogBuild = function (title,desc,submit,build,save,settings) {
      return oldDialog(title,desc,submit,(body,error) => {
        build(body,error);
        dialog.classList.add('sd-properties');
        const fields = body.querySelector('fieldset .fields');
        if (!fields) return;
        fields.classList.add('sd-properties-rows');
        const label = body.querySelector('input[aria-label="Display name"]');
        const hero = E('div','sd-properties-hero');
        const name = E('span','',label?.value||'New shortcut');
        const icon = E('div');
        hero.append(name,icon);
        body.prepend(hero);
        for (const event of ['input','change']) label?.addEventListener(event,()=>name.textContent=label.value||'New shortcut');
        const preview = body.querySelector('.sd-native-icon .inline > span');
        if (preview) {
          const update = () => icon.replaceChildren(...Array.from(preview.childNodes,n=>n.cloneNode(true)));
          const observer = new MutationObserver(update);
          observer.observe(preview,{childList:true,subtree:true});
          dialog.addEventListener('close',()=>observer.disconnect(),{once:true});
          update();
        }
      },save,settings);
    };
    try { return oldProperties(...args); }
    finally { dialogBuild=oldDialog; }
  };
  dialog.addEventListener('close',()=>{if(!dialog.open) dialog.classList.remove('sd-properties');});

  const basePositions = showPositions;
  showPositions = function (x,y) {
    if (phase !== 'closed') return basePositions(x,y);
    showMenu(x,y,()=>{
      mt('Screen position');
      const position = (edge,align) => {state.edge=edge;state.align=align;persist();refresh();};
      for (const edge of ['left','right','top','bottom']) {
        const b=mi(cap(edge)+' Screen Edge',()=>position(edge,state.align),{hint:state.edge===edge?'✓':''});
        b.setAttribute('aria-pressed',String(state.edge===edge));
      }
      sep();
      const names = vertical()?['Top','Center','Bottom']:['Left','Center','Right'];
      ['start','center','end'].forEach((a,i)=>{
        const b=mi('Align '+names[i],()=>position(state.edge,a),{hint:state.align===a?'✓':''});
        b.setAttribute('aria-pressed',String(state.align===a));
      });
    });
  };
  const oldDockMenu = dockMenu;
  dockMenu = function (...args) {
    const baseShow = showMenu;
    showMenu = function (x,y,build,...rest) {
      return baseShow(x,y,()=>{
        build();
        const button=menu.querySelector('button[aria-label="Position"]');
        if(button) {
          button.setAttribute('aria-label','Screen Position');
          if(button.firstChild?.nodeType===3) button.firstChild.textContent='Screen Position';
        }
      },...rest);
    };
    try { return oldDockMenu(...args); }
    finally { showMenu=baseShow; }
  };

  // A hold previews one new strip; a release commits it, and cancel restores the old grid.
  let edgeHold=null, strip=null, stripHint=null, dwellLock=null, pointer=null;
  const columnLimit=12, rowLimit=80;
  function stripBox() { return gsView.querySelector('.scroll')?.getBoundingClientRect(); }
  function edgeAt(x,y) {
    const r=stripBox();
    if(!r || x<r.left-8 || x>r.right+8 || y<r.top-8 || y>r.bottom+8) return null;
    const sides=[['left',Math.abs(x-r.left)],['right',Math.abs(x-r.right)],['top',Math.abs(y-r.top)],['bottom',Math.abs(y-r.bottom)]];
    const [side,dist]=sides.sort((a,b)=>a[1]-b[1])[0];
    return dist<=18?side:null;
  }
  const horizontalEdge = edge => edge==='left'||edge==='right';
  function clearHold() {
    if(edgeHold) clearTimeout(edgeHold.timer);
    edgeHold=null;
    stripHint?.remove(); stripHint=null;
  }
  function resetStrip(rollback) {
    clearHold();dwellLock=null;pointer=null;
    const old=strip;strip=null;
    if(rollback&&old) {
      old.f.cols=old.cols;old.f.rows=old.rows;
      for(const [item,r,c] of old.cells) {item.r=r;item.c=c;}
      if(phase==='open'&&active()===old.f) refresh();
    }
  }
  function showEdge(side) {
    const r=stripBox();
    if(!r)return;
    stripHint=E('div','sd-edge-grow');
    const w=horizontalEdge(side)?6:r.width, h=horizontalEdge(side)?r.height:6;
    Object.assign(stripHint.style,{left:(side==='right'?r.right-w:r.left)+'px',top:(side==='bottom'?r.bottom-h:r.top)+'px',width:w+'px',height:h+'px'});
    document.body.append(stripHint);
  }
  function grow(side,f) {
    if (!pointer || !drag || active()!==f || edgeAt(pointer.x,pointer.y)!==side) return;
    const before=locate(pointer.x,pointer.y);
    if(!before)return;
    strip={f,side,cols:f.cols,rows:f.rows,cells:f.slots.map(t=>[t,t.r,t.c]),x:pointer.x,y:pointer.y,
      landing:{r:side==='top'?0:side==='bottom'?f.rows:before.r,c:side==='left'?0:side==='right'?f.cols:before.c}};
    if(horizontalEdge(side)) {if(side==='left')f.slots.forEach(t=>t.c++);f.cols++;}
    else {if(side==='top')f.slots.forEach(t=>t.r++);f.rows++;}
    try { renderGrid(f);applyBox(gridBox(f));views(true);fitNative(); }
    catch(error){resetStrip(true); throw error;}
    const sc=gsView.querySelector('.scroll');
    if (side==='left') sc.scrollLeft=0;
    if (side==='right') sc.scrollLeft=sc.scrollWidth-sc.clientWidth;
    if (side==='top') sc.scrollTop=0;
    if (side==='bottom') sc.scrollTop=sc.scrollHeight-sc.clientHeight;
    mark(strip.landing);
  }
  document.addEventListener('pointermove',e=>{
    if(!drag || !press || e.pointerId!==press.id || phase!=='open') return;
    pointer={x:e.clientX,y:e.clientY};
    if(strip) { // The pointer can stay where the old edge was and still drop into the new strip.
      if(Math.hypot(pointer.x-strip.x,pointer.y-strip.y)<10) mark(strip.landing);
      return;
    }
    if(!e.buttons && e.pointerType==='mouse') { resetStrip(true);return; }
    const side=edgeAt(pointer.x,pointer.y),f=active();
    if(side && side===dwellLock)return;
    if(side!==dwellLock) dwellLock=null;
    if(!side || !f || (horizontalEdge(side)?f.cols>=columnLimit:f.rows>=rowLimit)) {
      clearHold(); return;
    }
    if (edgeHold?.side===side) return;
    clearHold();showEdge(side);
    try { if (!island.hasPointerCapture(e.pointerId)) island.setPointerCapture(e.pointerId); } catch {}
    edgeHold={side,timer:setTimeout(()=>{
      clearHold();dwellLock=side;grow(side,f);
    },1000)};
  });
  document.addEventListener('pointerup',e=>{
    if(!strip) {clearHold();dwellLock=null;return;}
    if(!press||e.pointerId!==press.id||!drag)return;
    const item=drag.item,f=strip.f;
    const near=Math.hypot(e.clientX-strip.x,e.clientY-strip.y)<10;
    const where=near?strip.landing:insideIsland(e.clientX,e.clientY)?locate(e.clientX,e.clientY):null;
    if(!where) {resetStrip(true);return;}
    e.stopImmediatePropagation();
    resetStrip(false);clearPointer();suppressClick=performance.now()+450;
    if(item.r===where.r&&item.c===where.c){persist();refresh();}
    else reposition(f,item,where);
  },true);
  for(const event of ['pointercancel','lostpointercapture']) document.addEventListener(event,()=>resetStrip(true),true);
  window.addEventListener('blur',()=>resetStrip(true));
  const originalClearPointer = clearPointer;
  clearPointer = function (...args) {resetStrip(true);return originalClearPointer(...args);};

  if(!invoke || !nativeAPI) return;
  let updating = false;
  let candidate = null;
  let lastPhaseChange = performance.now();
  const phaseObserver = new MutationObserver(() => {
    lastPhaseChange = performance.now();
    candidate = null;
  });
  phaseObserver.observe(island, {attributes:true,attributeFilter:['data-phase']});

  const sameArea = (a,b) => a && b &&
    ['px','py','w','h','scale'].every(k => a[k] === b[k]);
  const busy = () => nativeBusy || surfaceWanted || phase !== 'closed' || dialog.open || !menu.hidden;

  function usable(monitor, taskbars) {
    const wa=monitor.workArea,p=monitor.position || wa.position,z=monitor.size || wa.size;
    const bounds={l:p.x,t:p.y,r:p.x+z.width,b:p.y+z.height};
    const a={l:wa.position.x,t:wa.position.y,r:wa.position.x+wa.size.width,b:wa.position.y+wa.size.height};
    for(const q of taskbars) {
      const l=Math.max(q.left,bounds.l),r=Math.min(q.right,bounds.r),t=Math.max(q.top,bounds.t),b=Math.min(q.bottom,bounds.b);
      if (r<=l || b<=t) continue;
      if ((r-l)>=(b-t)) {
        if (b>=bounds.b-2 && t>bounds.t) a.b=Math.min(a.b,t);
        else if (t<=bounds.t+2 && b<bounds.b) a.t=Math.max(a.t,b);
      } else {
        if (r>=bounds.r-2 && l>bounds.l) a.r=Math.min(a.r,l);
        else if (l<=bounds.l+2 && r<bounds.r) a.l=Math.max(a.l,r);
      }
    }
    const scale=monitor.scaleFactor,pad=Math.ceil(8*scale);
    const w=Math.floor((a.r-a.l-2*pad)/scale),h=Math.floor((a.b-a.t-2*pad)/scale);
    return w>0&&h>0?shieldNativeArea({px:a.l+pad,py:a.t+pad,w,h,scale}):null;
  }

  async function applyArea(area) {
    const sizeChanged = area.w !== nativeArea.w || area.h !== nativeArea.h || area.scale !== nativeArea.scale;
    await invoke('set_dock_bounds', {
      x:area.px, y:area.py,
      width:Math.round(area.w*area.scale), height:Math.round(area.h*area.scale)
    });
    if (sizeChanged) await settleViewport(area);
    nativeArea=area;nativeHost={x:0,y:0,w:area.w,h:area.h};nativeRegion=null;
    setCaptionGutter(area);
    renderDock();applyBox(dockBox());views(false);
    await setNativeRegion(nativeCover(dockBox()));
  }

  async function updateArea(startup = false) {
    const tooSoon = () => !startup && performance.now()-lastPhaseChange < 250;
    if(updating || !fixedHost || !nativeWindow || busy() || tooSoon()) {
      candidate = null;
      return;
    }
    updating=true;
    try {
      const [monitor,bars]=await Promise.all([nativeAPI.currentMonitor(),invoke('dock_taskbar_rects',{})]);
      if(!monitor?.workArea || busy() || tooSoon()) {
        candidate = null;
        return;
      }
      const next=usable(monitor,bars);
      if(!next || sameArea(next,nativeArea)) {
        candidate = null;
        return;
      }
      // The hidden startup window is safe to move immediately. Once visible,
      // confirm a changed work area across two polls before touching the HWND.
      if(!startup && !sameArea(next,candidate)) {
        candidate = next;
        return;
      }
      candidate = null;
      const old=nativeArea;
      nativeBusy=true;
      try {
        await nativeQueue;
        await applyArea(next);
      } catch(error) {
        try { await applyArea(old); }
        catch (restoreError) { console.warn('Could not restore dock work area:',restoreError); }
        throw error;
      } finally {nativeBusy=false;}
    } finally {updating=false;}
  }
  // Fit the first region and reserve real taskbar space before revealing the window.
  async function bootWidget() {
    await nativeReady;
    if(!fixedHost) console.warn('Stack Dock: using the existing non-fixed window fallback.');
    try {await updateArea(true);} catch(error) {console.warn('Could not adjust for the taskbar:',error);}
    await invoke('start_desktop_widget',{});
    setInterval(()=>void updateArea().catch(error=>console.warn('Dock work area:',error)),1000);
  }
  void bootWidget().catch(error=>console.error('Stack Dock desktop widget:',error));
})();
