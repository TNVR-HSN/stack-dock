// Load once, after stack-dock-native.js. Works with the fixed-host Stack Dock.
(() => {
  'use strict';
  const invoke = window.__TAURI__?.core?.invoke;
  const style = document.createElement('style');
  style.textContent = `
#dockView .folder-btn,#gsView .tile{position:relative;padding:0}
#dockView .folder-btn{gap:0}
#dockView .folder-btn>.folder-label{line-height:14px;margin-top:3px}
#dockView .folder-btn:hover,#gsView .tile:hover,.sample-item:hover{background:none;position:relative;z-index:1}
#dockView .folder-btn>.art,#gsView .tile>.art,.sample-item>.art{position:relative;overflow:visible;z-index:1}
#dockView .folder-btn>.art>img,#dockView .folder-btn>.art>svg,#gsView .tile>.art>img,#gsView .tile>.art>svg,.sample-item>.art>img,.sample-item>.art>svg{border-radius:inherit}
.sample-item>.art::after{content:"";position:absolute;inset:calc(-1 * var(--highlight-area,0px));pointer-events:none;border-radius:var(--highlight-round,var(--ir,25%));opacity:0;background:var(--sd-highlight-fill,transparent);box-shadow:var(--sd-highlight-inset,inset 0 0 0 2px #fff);transition:opacity .12s}
.sample-item:hover>.art::after{opacity:calc(var(--sd-hover,.14) * var(--sd-highlight-enabled,1))}
#gsView .tile{gap:0;justify-content:flex-start}
#gsView.sd-labels .tile{display:grid;grid-template-columns:minmax(0,1fr);grid-template-rows:var(--sd-icon-size) var(--sd-reflect) var(--sd-label-space);row-gap:2px;align-content:start;justify-items:center}
#gsView.sd-labels .tile>.art{grid-row:1}
#gsView.sd-labels .tile>.icon-effects{grid-row:2}
#gsView.sd-labels .tile>.caption{grid-row:3;align-self:start;font-size:var(--sd-label-font);line-height:1.2;height:2.4em;max-height:2.4em;overflow:hidden;overflow-wrap:anywhere;word-break:break-word;text-overflow:ellipsis}
#iconHover{position:fixed;inset:0;z-index:12;pointer-events:none}
.sd-hover-ring,.sd-hover-art,.sd-hover-label{position:fixed;pointer-events:none}
.sd-hover-art{z-index:1}
.sd-hover-original{visibility:hidden}
.sd-hover-label{z-index:2;display:grid;gap:4px;width:max-content;text-align:center;padding:3px 8px;border-radius:5px;background:var(--sd-tip-bg,#130f19ed);color:#fff;overflow-wrap:anywhere;unicode-bidi:plaintext;text-shadow:0 1px 2px #000;font-size:var(--sd-tip-size,12px);line-height:1.25;font-weight:650}
.sd-hover-target{font-size:max(9px,calc(var(--sd-tip-size,12px) - 1px));font-weight:400;color:#eee1f4;white-space:pre-wrap;unicode-bidi:plaintext;max-height:40dvh;overflow:hidden}
 .sample-item>.art{transition:transform var(--sd-hover-duration,0ms) var(--sd-hover-easing,ease-out)}
.sample-item:hover>.art{transform:var(--sd-hover-transform,none)}
.sample-item .sd-hover-label{position:absolute;top:100%;width:max-content;max-width:155px;display:none}
.sample-item:hover>.sd-hover-label{display:grid}
.sd-edge-grow{position:fixed;z-index:51;pointer-events:none;background:#d99bff80;border:1px solid #ead1ff;border-radius:5px}
.launch-view,.launch-sample{display:flex;align-items:center;gap:var(--launch-gap,12px);padding:0 var(--launch-pad,16px);color:var(--launch-text,#fff);min-width:0}
.launch-view{position:absolute;inset:0;pointer-events:none}
.launch-sample{justify-content:flex-start;background:var(--launch-fill);max-width:100%;overflow:hidden}
.launch-copy{min-width:0;flex:1;display:grid;gap:2px}
.launch-title,.launch-caption{overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
.launch-title{font-size:var(--launch-font,19px);font-weight:650;letter-spacing:.03em}
.launch-caption{font-size:11px;opacity:.76}
.launch-settings .launch-sample{grid-column:1/-1}
`;
  document.head.append(style);

  const numeric = (v, lo, hi, fallback) => Number.isFinite(v) ? Math.max(lo, Math.min(hi, v)) : fallback;
  Object.assign(dockDefaults, {motionSpeed:100});
  const launchDefaults={launchEnabled:true,launchWidth:300,launchHeight:64,launchHold:1250,
    launchTravel:300,launchReturn:210,launchIcon:42,launchFont:19,launchRadius:32,
    launchOpacity:65,launchFollowDock:false,launchColor:'#111a2c',launchText:'#ffffff',launchLabel:true,launchGap:12,launchPad:16};
  Object.assign(dockDefaults,launchDefaults);
  const launchView=$('launchView'),launchGlass=$('launchGlass');
  function launchStyle(s){return {...launchDefaults,...s};}
  function launchSurface(s,dock=state.dock) {
    return s.launchFollowDock
      ? {color:dock.color??dockDefaults.color,opacity:dock.opacity??dockDefaults.opacity,border:false}
      : {color:s.launchColor,opacity:s.launchOpacity,border:false};
  }
  function launchContent(node,item,s,size,surface=launchSurface(s)) {
    node.style.setProperty('--launch-gap',s.launchGap+'px');
    node.style.setProperty('--launch-pad',s.launchPad+'px');
    node.style.setProperty('--launch-text',s.launchText);
    node.style.setProperty('--launch-font',s.launchFont+'px');
    node.style.setProperty('--launch-fill','rgba('+hexRgb(surface.color)+','+surface.opacity/100+')');
    const icon=shortcutArt(item,size,item.radius||'50%',{iconRotation:item.surfaceRotation});
    const copy=E('div','launch-copy'),title=E('div','launch-title',item.name);
    title.title=item.name;
    copy.append(title);
    if(s.launchLabel)copy.append(E('div','launch-caption','Launched'));
    node.replaceChildren(icon,copy);
    return {icon,copy};
  }
  function launchControls(body,s,readDock) {
    const panel=settingsCard(body,'Launch indicator preview','launch');
    panel.parentElement.classList.add('sd-preview-card');
    const enabled=check(panel,'Show launch indicator',s.launchEnabled);
    const label=check(panel,'Show Launched caption',s.launchLabel);
    const preview=E('div','launch-sample');panel.classList.add('launch-settings');panel.append(preview);
    const timing=settingsCard(body,'Motion and timing','launch');
    const frame=settingsCard(body,'Panel dimensions','launch');
    const style=settingsCard(body,'Typography and color','launch');
    const number=(card,key,label,min,max,unit)=>field(card,label,'range',s[key],{min,max,datasetUnit:unit,defaultValue:launchDefaults[key]});
    const inputs={
      launchWidth:number(frame,'launchWidth','Maximum width',140,680,' px'),
      launchHeight:number(frame,'launchHeight','Height',42,120,' px'),
      launchRadius:number(frame,'launchRadius','Panel corner radius',0,60,' px'),
      launchPad:number(frame,'launchPad','Side padding',4,28,' px'),
      launchHold:number(timing,'launchHold','Time shown',0,8000,' ms'),
      launchTravel:number(timing,'launchTravel','Icon glide duration',0,1000,' ms'),
      launchReturn:number(timing,'launchReturn','Return to dock duration',0,700,' ms'),
      launchIcon:number(style,'launchIcon','Icon size',16,84,' px'),
      launchFont:number(style,'launchFont','Title size',11,34,' px'),
      launchOpacity:number(style,'launchOpacity','Panel opacity',0,100,'%'),
      launchGap:number(style,'launchGap','Icon and text spacing',0,28,' px'),
    };
    const background=field(style,'Panel color','color',s.launchColor);
    const follow=check(style,'Follow dock background',s.launchFollowDock);
    follow.parentElement.classList.add('card-toggle');
    const text=field(style,'Text color','color',s.launchText);
    const read=()=>({launchEnabled:enabled.checked,launchLabel:label.checked,
      launchColor:background.value,launchText:text.value,launchFollowDock:follow.checked,
      ...Object.fromEntries(Object.entries(inputs).map(([k,v])=>[k,+v.value]))});
    const draw=()=>{
      const c=read();
      background.disabled=inputs.launchOpacity.disabled=c.launchFollowDock;
      preview.hidden=!c.launchEnabled;
      preview.style.width=c.launchWidth+'px';
      preview.style.height=c.launchHeight+'px';
      preview.style.borderRadius=c.launchRadius+'px';
      launchContent(preview,{kind:'program',name:'Example shortcut'},c,Math.min(c.launchIcon,c.launchHeight-10),launchSurface(c,readDock()));
    };
    for(const controls of [panel,timing,frame,style]){controls.addEventListener('input',draw);controls.addEventListener('change',draw);}
    draw();
    read.refresh=draw;
    return read;
  }
  generalLaunchControls=(body,s,readDock)=>launchControls(body,launchStyle(s),readDock);
  Object.assign(gsDefaults, {labelFont:11, labelWidth:76});
  // New appearance keys are preserved by the matching layout backup script.
  const baseAppearance = appearance;
  appearance = function (body, s, isGrid) {
    const read = baseAppearance(body, s, isGrid);
    const shapePanel=settingsCard(body,'Icon shape','layout');
    const panel=settingsCard(body,'Hover highlight','hover');
    const defaults=isGrid?gsDefaults:dockDefaults;
    const slider = (parent,key,name,min,max,unit,fallback) =>
      field(parent,name,'range',numeric(s[key],min,max,fallback),{min,max,datasetUnit:unit,defaultValue:defaults[key]??fallback});
    const highlightEnabled=check(panel,'Enable hover highlight',s.highlightEnabled!==false);
    highlightEnabled.parentElement.classList.add('card-toggle');
    const hover=slider(panel,'hoverOpacity','Highlight opacity',0,100,'%',40);
    const area=slider(panel,'highlightArea','Highlight area',0,32,' px',0);
    const fill=slider(panel,'highlightFill','Inside fill',0,100,'%',30);
    const fillStyle=select(panel,'Fill style',[[0,'Hard ring'],[1,'Soft Ring']],s.highlightSoft!==false?'1':'0');
    const hit=settingsCard(body,'Pointer detection','hover');
    const hitExpand=check(hit,'Extend hover and click area',s.hitExpand);
    hitExpand.parentElement.classList.add('card-toggle');
    const hitPadding=slider(hit,'hitPadding','Extra detection distance',0,48,' px',8);
    hitPadding.parentElement.style.gridColumn='1/-1';
    hit.append(E('p','note','Makes nearby space over the dock or grid count toward the nearest icon. An icon directly under the pointer takes priority.'));
    const caption=settingsCard(body,'Hover labels','hover');
    const showLabels=check(caption,'Show hover labels',s.hoverLabels!==false);
    caption.append(E('p','note',isGrid?'If icon labels are visible, hover only shows the delayed path or URL.':'If icon labels are visible, hover does not repeat the folder name.'));
    const background=check(caption,'Show label background',s.hoverLabelBackground!==false);
    const labelSize=slider(caption,'hoverLabelSize','Text size',9,28,' px',12);
    const localDelay=isGrid?check(caption,'Override General details delay',s.customHoverDelay):null;
    const delay=isGrid?slider(caption,'hoverDetailsDelay','Details delay',0,5000,' ms',1500):null;
    if(delay)delay.step=100;
    const updateHover=()=>{
      hitPadding.disabled=!hitExpand.checked;
      background.disabled=labelSize.disabled=!showLabels.checked;
      if(localDelay){localDelay.disabled=!showLabels.checked;delay.disabled=!showLabels.checked||!localDelay.checked;}
    };
    for(const toggle of [hitExpand,showLabels,localDelay])toggle?.addEventListener('change',updateHover);
    updateHover();
    const reflect = settingsCard(body,'Reflection','effects');
    const reflected = check(reflect,'Show icon reflections',s.reflections);reflected.parentElement.classList.add('card-toggle');
    const reflectionIntensity = slider(reflect,'reflectionIntensity','Reflection intensity',0,100,'%',55);
    const reflectionArea = slider(reflect,'reflectionArea','Reflection height',0,64,' px',10);
    const depth = settingsCard(body,'Ground shadow','effects');
    const shadow = check(depth,'Show icon ground shadows',s.groundShadow);shadow.parentElement.classList.add('card-toggle');
    const groundShadowIntensity = slider(depth,'groundShadowIntensity','Shadow opacity',0,100,'%',65);
    const groundShadowOffset = slider(depth,'groundShadowOffset','Vertical shadow offset',-24,48,' px',0);
    depth.append(E('p','note','Negative moves the shadow up. Positive moves it down.'));
    const updateDepth = () => {
      reflectionIntensity.disabled=reflectionArea.disabled=!reflected.checked;
      groundShadowIntensity.disabled=groundShadowOffset.disabled=!shadow.checked;
    };
    reflected.addEventListener('change',updateDepth);shadow.addEventListener('change',updateDepth);updateDepth();
    const motion=settingsCard(body,'Hover motion','effects');
    const mode=select(motion,'Hover effect',[[0,'None'],[1,'Enlarge'],[2,'Lift'],[3,'Enlarge + lift'],
      [4,'Tilt'],[5,'Enlarge + lift + tilt']],String(s.hoverMode??0));
    const scale=slider(motion,'hoverScale','Enlarge to',100,200,'%',125);
    const lift=slider(motion,'hoverLift','Lift distance',0,48,' px',10);
    const angle=slider(motion,'hoverAngle','Tilt angle',-30,30,'°',10);
    const duration=slider(motion,'hoverDuration','Duration',0,900,' ms',160);
    const curve=select(motion,'Motion curve',[[0,'Linear'],[1,'Ease out'],[2,'Ease in/out'],[3,'Spring']],String(s.hoverCurve??1));
    const syncMotion=()=>{
      const m=+mode.value;
      scale.disabled=![1,3,5].includes(m);lift.disabled=![2,3,5].includes(m);angle.disabled=![4,5].includes(m);
      duration.disabled=curve.disabled=!m;
    };
    mode.addEventListener('change',syncMotion);syncMotion();
    const effects = () => ({highlightEnabled:highlightEnabled.checked,highlightArea:+area.value,
      highlightFill:+fill.value,highlightSoft:fillStyle.value==='1',
      hitExpand:hitExpand.checked,hitPadding:+hitPadding.value,hoverLabels:showLabels.checked,
      hoverLabelBackground:background.checked,hoverLabelSize:+labelSize.value,
      ...(isGrid?{customHoverDelay:localDelay.checked,hoverDetailsDelay:+delay.value}:{}),reflections:reflected.checked,
      reflectionIntensity:+reflectionIntensity.value,reflectionArea:+reflectionArea.value,
      groundShadow:shadow.checked,groundShadowIntensity:+groundShadowIntensity.value,groundShadowOffset:+groundShadowOffset.value,
      hoverMode:+mode.value,hoverScale:+scale.value,hoverLift:+lift.value,hoverAngle:+angle.value,
      hoverDuration:+duration.value,hoverCurve:+curve.value});
    const initial = isGrid?gridRoundness(s):dockRoundness(s);
    const shape = (parent,name,value) => field(parent,name,'range',value,
      {min:0,max:50,datasetUnit:'%',defaultValue:isGrid?gridRoundness(gsDefaults):dockRoundness(dockDefaults),displayScale:2});
    const corner=shape(shapePanel,'Icon roundness',initial);
    const rotation=rotationSlider(shapePanel,'Icon rotation',s.iconRotation);
    const highlight=shape(panel,'Highlight roundness',iconCornerValue(s.highlightCorner,initial));
    const syncHighlight=()=>{hover.disabled=area.disabled=highlight.disabled=fill.disabled=fillStyle.disabled=!highlightEnabled.checked;};
    highlightEnabled.addEventListener('change',syncHighlight);syncHighlight();
    let separate=Number.isFinite(s.highlightCorner);
    highlight.addEventListener('input',()=>{separate=true;});
    corner.addEventListener('input',()=>{if(!separate){highlight.value=corner.value;highlight.oninput();}});
    const corners=()=>({iconCorner:+corner.value,iconRotation:+rotation.value,highlightCorner:separate?+highlight.value:undefined,roundIcons:undefined});
    if (isGrid) {
      const labels=settingsCard(body,'Label dimensions','layout');
      const font=slider(labels,'labelFont','Label size',9,18,' px',11);
      const width=slider(labels,'labelWidth','Minimum label width',64,200,' px',76);
      return () => ({...read(),labelFont:+font.value,labelWidth:+width.value,hoverOpacity:+hover.value,...corners(),...effects()});
    }
    return () => ({...read(),hoverOpacity:+hover.value,...corners(),...effects()});
  };

  // Detached hover layer avoids clipping at the edge of either scroll container.
  const hoverLayer=$('iconHover');
  const svgNS='http://www.w3.org/2000/svg',svgNode=name=>document.createElementNS(svgNS,name);
  let hovered=null, hoverTimer=0, hoverAnimation=null, hoverLayout=null, hoveredArt=null;
  const baseFitNative=fitNative;
  fitNative=function(grid){
    if(!hovered||hoverLayer.hidden||!fixedHost||!nativeArea||!hoverLayout||nativeBusy||surfaceWanted||!menu.hidden||dialog.open){
      return baseFitNative(grid);
    }
    return moveHost(nativeCover(readBox(),hoverLayout));
  };
  clearHover=function(refit=true){
    const wasVisible=!hoverLayer.hidden;
    hovered=null;clearTimeout(hoverTimer);hoverTimer=0;hoverAnimation?.cancel();hoverAnimation=null;
    hoveredArt?.classList.remove('sd-hover-original');hoveredArt=null;hoverLayout=null;
    hoverLayer.hidden=true;hoverLayer.replaceChildren();
    if(refit&&wasVisible)fitNative();
  };
  const hoverEasings=['linear','ease-out','ease-in-out','cubic-bezier(.2,1.27,.34,1)'];
  function hoverMotion(s,k=1){
    if(reduced.matches)return {scale:1,y:0,angle:0,time:0,easing:'linear'};
    const mode=s.hoverMode??0;
    return {
      scale:[1,3,5].includes(mode)?(s.hoverScale??125)/100:1,
      y:[2,3,5].includes(mode)?-(s.hoverLift??10)*k:0,
      angle:[4,5].includes(mode)?(s.hoverAngle??10):0,
      time:s.hoverDuration??160,easing:hoverEasings[s.hoverCurve??1]
    };
  }
  const matrix=m=>`translateY(${m.y}px) rotate(${m.angle}deg) scale(${m.scale})`;
  stylePreviewIcon=(item,s,k,name)=>{
    const m=hoverMotion(s,k);
    item.style.setProperty('--sd-hover-transform',matrix(m));
    item.style.setProperty('--sd-hover-duration',m.time+'ms');
    item.style.setProperty('--sd-hover-easing',m.easing);
    if(name&&showHoverName(s,state.general,s.hoverLabels!==false)){
      const label=E('span','sd-hover-label',name);styleHoverLabel(label,s);item.append(label);
    }
  };
  const place=(el,x,y,w,h)=>{
    el.style.left=x+'px';el.style.top=y+'px';
    if(w!==undefined)el.style.width=w+'px';
    if(h!==undefined)el.style.height=h+'px';
  };
  function styleHoverLabel(label,s){
    label.style.setProperty('--sd-tip-size',(s.hoverLabelSize??12)+'px');
    if(s.hoverLabelBackground===false)label.style.setProperty('--sd-tip-bg','transparent');
  }
  function showIconHover(button){
    const onDock=phase==='closed'&&dockView.contains(button);
    if(!onDock&&!(phase==='open'&&gsView.contains(button)))return;
    if(dialog.open||!menu.hidden||surfaceWanted||drag||nativeBusy||hovered===button)return;
    const entry=onDock?dockEntryAt(button):active()?.slots.find(t=>t.id===button.dataset.tile);
    const s=onDock?state.dock:active()?.style, source=button.querySelector(':scope > .art');
    if(!entry||!s||!source)return;
    const r=source.getBoundingClientRect();
    if(!r.width||!r.height)return;
    clearHover(false);
    hovered=button;
    const d=dims(), top=d.top-nativeHost.y, left=-nativeHost.x, right=d.w-nativeHost.x, bottom=top+d.h;
    const m=hoverMotion(s);
    const moved=m.scale!==1||m.y!==0||m.angle!==0;
    const angle=Math.abs(m.angle)*Math.PI/180;
    const extent=r.width*m.scale*(Math.cos(angle)+Math.sin(angle));
    const growth=Math.max(0,(extent-r.width)/2);
    const cx=clamp(r.left+r.width/2,left+extent/2,right-extent/2);
    const cy=clamp(r.top+r.height/2+m.y,top+extent/2,bottom-extent/2);
    const xShift=cx-r.left-r.width/2;
    m.y=cy-r.top-r.height/2;
    const area=s.highlightArea??0,alpha=s.highlightEnabled===false?0:(s.hoverOpacity??40)/100;
    const ring=E('div','sd-hover-ring');
    const x1=clamp(r.left-area,left,right), y1=clamp(r.top-area,top,bottom);
    const x2=clamp(r.right+area,left,right), y2=clamp(r.bottom+area,top,bottom);
    const w=x2-x1,h=y2-y1;
    let lowX=Math.min(x1,r.left+xShift-growth),lowY=Math.min(y1,r.top+m.y-growth);
    let highX=Math.max(x2,r.right+xShift+growth),highY=Math.max(y2,r.bottom+m.y+growth);
    if(alpha>0&&w>0&&h>0){
      place(ring,x1,y1,w,h);
      const svg=svgNode('svg'),mask=svgNode('mask'),background=svgNode('rect'),paint=svgNode('g');
      const holes='sd-hover-outline-mask';
      const attrs=(el,values)=>Object.entries(values).forEach(([k,v])=>el.setAttribute(k,v));
      attrs(svg,{width:w,height:h,viewBox:`0 0 ${w} ${h}`});
      attrs(mask,{id:holes,maskUnits:'userSpaceOnUse',x:0,y:0,width:w,height:h});
      attrs(background,{width:w,height:h,fill:'white'});mask.append(background);
      const siblings=(onDock?dockView:gsView).querySelectorAll('button > .art');
      for(const sibling of siblings){
        if(sibling===source)continue;
        const b=sibling.getBoundingClientRect();
        if(b.right<=x1||b.left>=x2||b.bottom<=y1||b.top>=y2)continue;
        const cut=svgNode('rect');
        const ownRadius=parseFloat(sibling.style.getPropertyValue('--ir'))||0;
        attrs(cut,{x:b.left-x1,y:b.top-y1,width:b.width,height:b.height,rx:b.width*ownRadius/100,fill:'black'});
        mask.append(cut);
      }
      const pc=parseFloat(source.style.getPropertyValue('--highlight-round'))||0;
      const shape={x:1,y:1,width:Math.max(0,w-2),height:Math.max(0,h-2),rx:Math.min(w,h)*pc/100};
      const fill=clamp((s.highlightFill??30)/100,0,1);
      const defs=svgNode('defs');defs.append(mask);
      attrs(paint,{mask:'url(#'+holes+')',opacity:alpha});
      const outline=svgNode('rect');
      if(s.highlightSoft!==false){
        const clip=svgNode('clipPath'),clipShape=svgNode('rect');
        attrs(clip,{id:'sd-hover-airbrush-clip'});attrs(clipShape,shape);
        clip.append(clipShape);defs.append(clip);
        const fog=svgNode('g');attrs(fog,{'clip-path':'url(#sd-hover-airbrush-clip)'});
        const depth=Math.min(w,h)*(.12+fill*.9);
        // Wider strokes paint first. Narrower layers build opacity near the rim.
        for(let i=9;i>=1;i--){
          const layer=svgNode('rect');
          attrs(layer,{...shape,fill:'none',stroke:'white','stroke-width':Math.max(1,depth*i/9),
            'stroke-opacity':(.055+fill*.115)*(1.4-i/10)});
          fog.append(layer);
        }
        paint.append(fog);
        attrs(outline,{...shape,fill:'none',stroke:'white','stroke-opacity':.3,'stroke-width':1});
      }else{
        attrs(outline,{...shape,fill:'white','fill-opacity':fill,stroke:'white','stroke-width':2});
      }
      paint.append(outline);svg.append(defs,paint);ring.append(svg);hoverLayer.append(ring);
    }
    let clone;
    if(moved){
      clone=source.cloneNode(true);
      clone.classList.add('sd-hover-art');
      place(clone,r.left+xShift,r.top,r.width,r.height);
      clone.style.transform=matrix(m);hoverLayer.append(clone);
      source.classList.add('sd-hover-original');hoveredArt=source;
    }
    const general={...generalDefaults,...state.general};
    const label=E('div','sd-hover-label'), title=E('span','sd-hover-name',entry.name);
    const trail=E('span','sd-hover-target',entry.target||'');
    trail.hidden=true;label.append(title,trail);
    styleHoverLabel(label,s);
    const labelsEnabled=s.hoverLabels!==false&&(!onDock||entry.folderHoverLabels!==false);
    const showDetails=labelsEnabled&&general.hoverTargets&&!!entry.target;
    title.hidden=!showHoverName(s,general,labelsEnabled);
    if(!title.hidden||showDetails)hoverLayer.append(label);
    hoverLayer.hidden=false;
    function layout(){
      if(hovered!==button)return;
      if(label.isConnected&&(!title.hidden||!trail.hidden)){
        label.style.maxWidth=Math.min(540,d.w-12)+'px';
        const bounds=label.getBoundingClientRect();
        const x=clamp(r.left+(r.width-bounds.width)/2,left+6,Math.max(left+6,right-bounds.width-6));
        const caption=button.querySelector(':scope > .caption,:scope > .folder-label');
        const below=Math.max(r.bottom,caption?.getBoundingClientRect().bottom??r.bottom)+5;
        // If there is no room below the icon, keep the entire caption above it.
        const y=Math.max(top+4,below+bounds.height+4<=bottom?below:Math.min(r.top-5-bounds.height,bottom-bounds.height-4));
        place(label,x,y);
        lowX=Math.min(lowX,x);lowY=Math.min(lowY,y);highX=Math.max(highX,x+bounds.width);highY=Math.max(highY,y+bounds.height);
      }else{label.style.visibility='hidden';}
      if(!title.hidden||!trail.hidden)label.style.visibility='';
      hoverLayout={x:lowX+nativeHost.x,y:lowY+nativeHost.y,w:highX-lowX,h:highY-lowY};
      fitNative();
    }
    layout();
    if(clone?.animate&&m.time){
      hoverAnimation=clone.animate([{transform:'none'},{transform:matrix(m)}],{duration:m.time,easing:m.easing});
    }
    if(showDetails){
      const delay=s.customHoverDelay?(s.hoverDetailsDelay??1500):general.hoverDelay;
      const details=()=>{if(hovered===button){trail.hidden=false;layout();}};
      if(delay===0)details();
      else hoverTimer=setTimeout(details,delay);
    }
  }
  const hitIcon=el=>el?.closest?.('.folder-btn,.tile');
  expandedIconAt=event=>{
    const {clientX:x,clientY:y,target}=event;
    if(target?.closest?.('button')||!insideIsland(x,y))return null;
    const view=phase==='closed'?dockView:phase==='open'?gsView:null;
    const appearance=phase==='closed'?state.dock:active()?.style;
    if(!view||!appearance?.hitExpand||!view.contains(target))return null;
    const scroll=view.querySelector('.scroll,.folders');
    if(!scroll)return null;
    const clip=scroll.getBoundingClientRect();
    // Do not make scrollbars behave like shortcut buttons.
    if(target===scroll&&((scroll.scrollWidth>scroll.clientWidth&&y>=clip.top+scroll.clientHeight)||
      (scroll.scrollHeight>scroll.clientHeight&&x>=clip.left+scroll.clientWidth)))return null;
    const reach=appearance.hitPadding??8;
    let best=null,bestDistance=Infinity;
    for(const button of view.querySelectorAll('.folder-btn,.tile')){
      const r=button.getBoundingClientRect();
      if(r.left>=clip.right||r.top>=clip.bottom||r.right<=clip.left||r.bottom<=clip.top)continue;
      const dx=Math.max(Math.max(r.left,clip.left)-x,0,x-Math.min(r.right,clip.right));
      const dy=Math.max(Math.max(r.top,clip.top)-y,0,y-Math.min(r.bottom,clip.bottom));
      const distance=dx*dx+dy*dy;
      if(distance<=reach*reach&&distance<bestDistance){best=button;bestDistance=distance;}
    }
    return best;
  };
  function hoverPointer(e){
    if(e.pointerType==='touch'||e.buttons||drag)return;
    const icon=hitIcon(e.target)||expandedIconAt(e);
    if(icon)showIconHover(icon);
    else if(hovered)clearHover();
  }
  island.addEventListener('pointerover',hoverPointer);
  island.addEventListener('pointermove',hoverPointer);
  island.addEventListener('pointerout',e=>{if(!island.contains(e.relatedTarget))clearHover();});
  island.addEventListener('click',e=>{
    if(!e.detail||e.button!==0||performance.now()<suppressClick||dialog.open||!menu.hidden)return;
    const icon=expandedIconAt(e);
    if(!icon)return;
    e.preventDefault();icon.dispatchEvent(new MouseEvent('click',{bubbles:true,detail:e.detail,clientX:e.clientX,clientY:e.clientY}));
  });
  island.addEventListener('pointerdown',()=>clearHover(),true);
  island.addEventListener('scroll',()=>clearHover(),true);
  window.addEventListener('blur',()=>clearHover());

  const baseMetrics = metrics;
  let measuring;
  metrics = function (folder) {
    const m = baseMetrics(folder), s = folder.style;
    if (!s.labels) return m;
    const font = numeric(s.labelFont,9,18,11);
    const height=s.icon+effectsHeight(s,s.icon)+Math.ceil(font*2.4)+4;
    if(m.gapX===0)return {...m,ch:height};
    const minWidth = Math.max(m.cw,numeric(s.labelWidth,64,200,76));
    try { measuring ||= document.createElement('canvas').getContext('2d'); } catch {}
    if(measuring)measuring.font=font+'px "Segoe UI",system-ui,sans-serif';
    const widths=new Map();
    const widthOf=label=>{
      if(!widths.has(label))widths.set(label,measuring?measuring.measureText(label).width:Array.from(label).length*font*.57);
      return widths.get(label);
    };
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
    return {...m,cw:w,ch:height};
  };
  const baseRenderGrid = renderGrid;
  renderGrid = function (folder,...args) {
    const s = folder.style;
    gsView.classList.toggle('sd-labels',!!s.labels);
    gsView.style.setProperty('--sd-icon-size',s.icon+'px');
    gsView.style.setProperty('--sd-reflect',effectsHeight(s,s.icon)+'px');
    const font = numeric(s.labelFont,9,18,11);
    gsView.style.setProperty('--sd-label-font',font+'px');
    gsView.style.setProperty('--sd-label-space',Math.ceil(font*2.4)+'px');
    gsView.style.setProperty('--sd-hover',numeric(s.hoverOpacity,0,100,40)/100);
    return baseRenderGrid(folder,...args);
  };
  const baseRenderDock = renderDock;
  renderDock = function (...args) {
    dockView.style.setProperty('--sd-hover',numeric(state.dock.hoverOpacity,0,100,40)/100);
    return baseRenderDock(...args);
  };

  showLaunchIndicator = async function(item,start,sourceArt,source,fromDock=false) {
    const s=launchStyle(state.dock);
    if(!s.launchEnabled||phase!==(fromDock?'closed':'open')||nativeBusy)return false;
    closeMenu();clearPointer();
    const from=readBox(),dock=dockBox(),radius=fromDock?dockIconRadius(item):gridIconRadius(active().style);
    const oldView=fromDock?dockView:gsView,oldGlass=fromDock?dockGlass:gsGlass;
    const target=box(s.launchWidth,s.launchHeight,s.launchRadius);
    const moving=state.dock.motion&&!reduced.matches&&!!island.animate;
    const duration=moving?s.launchTravel:0,returnTime=moving?s.launchReturn:0;
    nativeBusy=true;setPhase('launching');
    const play=async(animations)=>{await Promise.allSettled(animations.map(a=>a.finished));};
    const fade=(el,a,b,ms)=>el.animate([{opacity:a},{opacity:b}],{duration:ms,fill:'both',easing:'ease-out'});
    const hull=(a,b,ms)=>island.animate([boxFrame(a),boxFrame(b)],{duration:ms,fill:'both',easing:'cubic-bezier(.22,.74,.23,1)'});
    const release=animations=>animations.forEach(a=>a.cancel());
    let flying,incoming=[],outgoing=[];
    try {
      if(nativeWindow&&!await moveHost(nativeCover(from,target,dock)))return true;
      styleGlass(launchGlass,launchSurface(s));
      island.style.boxShadow='none';
      launchGlass.hidden=false;launchView.hidden=false;
      const surfaceRotation=(fromDock?state.dock:active()?.style)?.iconRotation;
      const {icon,copy}=launchContent(launchView,{...item,radius,surfaceRotation},s,Math.max(12,Math.min(s.launchIcon,target.h-10,target.w-s.launchPad*2-24)));
      applyBox(target);
      if(duration){
        const dest=icon.getBoundingClientRect();
        incoming=[hull(from,target,duration),fade(oldView,1,0,duration*.65),fade(oldGlass,1,0,duration),
          fade(launchGlass,0,1,duration),fade(copy,0,1,duration)];
        if(start?.width&&dest.width&&sourceArt){
          icon.style.visibility='hidden';
          if(source)source.style.visibility='hidden';
          flying=E('div','folder-flight');
          flying.style.left=start.left+'px';flying.style.top=start.top+'px';
          flying.append(sourceArt);desktop.append(flying);
          const scale=dest.width/start.width;
          const dx=dest.left-start.left+(dest.width-start.width)/2,dy=dest.top-start.top+(dest.height-start.height)/2;
          incoming.push(flying.animate([{transform:'translate(0px,0px) scale(1)'},
            {transform:'translate('+dx+'px,'+dy+'px) scale('+scale+')'}],{duration,easing:'cubic-bezier(.22,.74,.23,1)',fill:'both'}));
        }
        await play(incoming);
      }
      oldView.style.opacity=oldGlass.style.opacity='0';
      icon.style.visibility='';flying?.remove();release(incoming);incoming=[];
      setPhase('launched');
      if(nativeWindow)await moveHost(nativeCover(target));
      if(s.launchHold)await new Promise(resolve=>setTimeout(resolve,s.launchHold));
      setPhase('returning');
      if(nativeWindow&&!await moveHost(nativeCover(target,dock)))return true;
      applyBox(dock);views(false);
      if(returnTime){
        outgoing=[hull(target,dock,returnTime),fade(launchGlass,1,0,returnTime),
          fade(launchView,1,0,returnTime*.7),fade(dockGlass,0,1,returnTime),fade(dockView,0,1,returnTime)];
        await play(outgoing);
      }
      return true;
    } catch(error) {
      console.warn('Stack Dock launch indicator:',error);
      return true;
    } finally {
      release(incoming);release(outgoing);flying?.remove();
      if(source)source.style.visibility='';
      launchView.hidden=launchGlass.hidden=true;launchView.replaceChildren();
      nativeBusy=false;finishClose(false);
    }
  };

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
    try { renderGrid(f);const grid=gridBox(f,currentMetrics);applyBox(grid);views(true);fitNative(grid); }
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
  const busy = () => nativeBusy || surfaceWanted || !!drag || externalDropActive || phase !== 'closed' || dialog.open || !menu.hidden;

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
    const scale=monitor.scaleFactor;
    const w=(a.r-a.l)/scale,h=(a.b-a.t)/scale;
    return w>0&&h>0?shieldNativeArea({px:a.l,py:a.t,w,h,scale}):null;
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
    scheduleDropZone();
    setInterval(()=>void updateArea().catch(error=>console.warn('Dock work area:',error)),1000);
  }
  void bootWidget().catch(error=>console.error('Stack Dock desktop widget:',error));
})();
