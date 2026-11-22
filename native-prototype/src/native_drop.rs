use super::{APP,Handle,Point};
use std::{cell::Cell,path::Path,ptr};
use windows::{core::{implement,Ref,Result},Win32::{Foundation::{HWND,POINTL},
    System::{Com::{CoTaskMemFree,DVASPECT_CONTENT,FORMATETC,IDataObject,TYMED_HGLOBAL},
    Ole::{DROPEFFECT,DROPEFFECT_COPY,DROPEFFECT_LINK,DROPEFFECT_NONE,IDropTarget,IDropTarget_Impl,OleInitialize,OleUninitialize,RegisterDragDrop,ReleaseStgMedium,RevokeDragDrop},SystemServices::MODIFIERKEYS_FLAGS},
    UI::Shell::{DragQueryFileW,HDROP,IShellItemArray,SHCreateShellItemArrayFromDataObject,SIGDN_FILESYSPATH}}};
fn file_format() -> FORMATETC {
    const CF_HDROP: u16 = 15;
    FORMATETC {
        cfFormat: CF_HDROP,
        ptd: ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

fn hdrop_paths(data: &IDataObject) -> Vec<String> {
    let Ok(mut medium) = (unsafe { data.GetData(&file_format()) }) else { return Vec::new(); };
    let mut paths = Vec::new();
    if medium.tymed == TYMED_HGLOBAL.0 as u32 {
        let handle = unsafe { medium.u.hGlobal };
        if !handle.0.is_null() {
            let drop = HDROP(handle.0);
            let count = unsafe { DragQueryFileW(drop, u32::MAX, None) };
            for i in 0..count.min(64) {
                let len = unsafe { DragQueryFileW(drop, i, None) };
                if len == 0 || len > 32768 { continue; }
                let mut buffer = vec![0; len as usize + 1];
                let written = unsafe { DragQueryFileW(drop, i, Some(&mut buffer)) };
                if written == 0 || written > len { continue; }
                let Ok(path) = String::from_utf16(&buffer[..written as usize]) else { continue; };
                if Path::new(&path).is_absolute() { paths.push(path); }
            }
        }
    }
    unsafe { ReleaseStgMedium(&mut medium); }
    paths
}

fn paths_from(data: &IDataObject) -> Vec<String> {
    let paths = hdrop_paths(data);
    if !paths.is_empty() { return paths; }
    // Desktop shell items (including some shortcuts) may offer a shell item
    // array instead of a conventional file drop list.
    let Ok(items) = (unsafe { SHCreateShellItemArrayFromDataObject::<_, IShellItemArray>(data) }) else {
        return Vec::new();
    };
    let count = unsafe { items.GetCount() }.unwrap_or(0);
    let mut paths = Vec::new();
    for i in 0..count.min(64) {
        let Ok(item) = (unsafe { items.GetItemAt(i) }) else { continue; };
        let Ok(value) = (unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }) else { continue; };
        let path = unsafe { value.to_string() };
        unsafe { CoTaskMemFree(Some(value.0.cast())); }
        if let Ok(path) = path {
            if Path::new(&path).is_absolute() { paths.push(path); }
        }
    }
    paths
}

// Never tell Explorer to MOVE a source file. StackDock only stores a reference.
fn safe_effect(allowed: DROPEFFECT) -> DROPEFFECT {
    if allowed.0 & DROPEFFECT_COPY.0 != 0 { DROPEFFECT_COPY }
    else if allowed.0 & DROPEFFECT_LINK.0 != 0 { DROPEFFECT_LINK }
    else { DROPEFFECT_NONE }
}


fn links(source:&IDataObject)->Vec<String>{
    use windows::Win32::System::Memory::{GlobalSize,GlobalLock,GlobalUnlock};
    let mut fmt=file_format();fmt.cfFormat=13;
    let Ok(mut medium)=(unsafe{source.GetData(&fmt)}) else{return vec![]};
    let mut out=Vec::new();
    unsafe{
        if medium.tymed==TYMED_HGLOBAL.0 as u32{
            let global=medium.u.hGlobal;let n=GlobalSize(global)/2;
            if n>0 && n<=256*1024{
                let p=GlobalLock(global) as *const u16;
                if !p.is_null(){let chars=std::slice::from_raw_parts(p,n);
                    let text=String::from_utf16_lossy(&chars[..chars.iter().position(|v|*v==0).unwrap_or(n)]);
                    out.extend(text.lines().map(str::trim).filter(|s|s.starts_with("https://")||s.starts_with("http://")).take(64).map(str::to_owned));
                    let _=GlobalUnlock(global);
                }
            }
        }
        ReleaseStgMedium(&mut medium);
    }
    out
}
#[implement(IDropTarget)]
struct Target {allowed:Cell<DROPEFFECT>}
fn feedback(p:Option<&POINTL>)->bool{
    APP.with(|c|{
        let Ok(mut state)=c.try_borrow_mut()else{return false};let Some(a)=state.as_mut()else{return false};
        let pt=p.filter(|p|a.settings.is_some() && a.indicator.is_none() && !a.settings_open && a.motion.is_none() && a.context(&Point{x:p.x,y:p.y})["valid"]==true);
        a.drag_at=pt.map(|p|(p.x,p.y));
        if pt.is_some(){let _=a.drag_tick();}else{a.dwell=None;}let _=a.draw();
        pt.is_some()
    })
}
#[allow(non_snake_case)]
impl IDropTarget_Impl for Target_Impl{
    fn DragEnter(&self,data:Ref<IDataObject>,_:MODIFIERKEYS_FLAGS,p:&POINTL,e:*mut DROPEFFECT)->Result<()>{
        let Some(e)=(unsafe{e.as_mut()})else{return Ok(())};
        let supported=data.as_ref().is_some_and(|s|(unsafe{s.QueryGetData(&file_format()).is_ok()})||!paths_from(s).is_empty()||!links(s).is_empty());
        self.allowed.set(if supported{safe_effect(*e)}else{DROPEFFECT_NONE});*e=if supported&&feedback(Some(p)){self.allowed.get()}else{DROPEFFECT_NONE};Ok(())
    }
    fn DragOver(&self,_:MODIFIERKEYS_FLAGS,p:&POINTL,e:*mut DROPEFFECT)->Result<()>{
        if let Some(e)=unsafe{e.as_mut()}{*e=if self.allowed.get().0!=0&&feedback(Some(p)){self.allowed.get()}else{DROPEFFECT_NONE};}Ok(())
    }
    fn DragLeave(&self)->Result<()>{self.allowed.set(DROPEFFECT_NONE);feedback(None);Ok(())}
    fn Drop(&self,data:Ref<IDataObject>,_:MODIFIERKEYS_FLAGS,p:&POINTL,e:*mut DROPEFFECT)->Result<()>{
        let Some(e)=(unsafe{e.as_mut()})else{return Ok(())};let allowed=self.allowed.replace(DROPEFFECT_NONE);*e=DROPEFFECT_NONE;
        if allowed.0!=0&&feedback(Some(p)){if let Some(s)=data.as_ref(){
            let paths=paths_from(s);let urls=if paths.is_empty(){links(s)}else{vec![]};
            if !paths.is_empty()||!urls.is_empty(){
                let count = paths.len();
                let direct = urls.is_empty() && !paths.iter().any(|path| {
                    Path::new(path).extension().is_some_and(|extension|
                        extension.eq_ignore_ascii_case("lnk") || extension.eq_ignore_ascii_case("url"))
                });
                let result = APP.with(|cell| {
                    let mut state = cell.borrow_mut();
                    let app = state.as_mut().ok_or("Dock closed")?;
                    let mut ctx = app.context(&Point { x: p.x, y: p.y });
                    if ctx["valid"] != true {
                        return Err(String::from("Drop the files inside StackDock."));
                    }
                    if direct {
                        app.add_dropped_paths(&ctx, &paths).map(|added| {
                            (added < count).then(|| format!(
                                "{added} shortcuts added. Check the available space and targets."
                            ))
                        })
                    } else {
                        ctx["action"] = "drop".into();
                        ctx["paths"] = paths.into();
                        ctx["urls"] = urls.into();
                        app.command(ctx).map(|()| None)
                    }
                });
                match result {
                    Ok(notice) => {
                        *e = allowed;
                        if let Some(notice) = notice {
                            super::message(ptr::null_mut(), &notice);
                        }
                    }
                    Err(error) => super::message(ptr::null_mut(), &error),
                }
            }
        }}
        if e.0!=0{APP.with(|c|{if let Some(a)=c.borrow().as_ref(){unsafe{super::KillTimer(a.hwnd,6)};}});}
        feedback(None);Ok(())
    }
}
pub(super) struct Registration(HWND);
pub(super) fn register(handle:Handle)->std::result::Result<Registration,String>{
    let hwnd=HWND(handle);unsafe{OleInitialize(None)}.map_err(|e|e.to_string())?;
    let target:IDropTarget=Target{allowed:Cell::new(DROPEFFECT_NONE)}.into();
    if let Err(e)=unsafe{RegisterDragDrop(hwnd,&target)}{unsafe{OleUninitialize()};return Err(e.to_string());}
    Ok(Registration(hwnd))
}
impl Drop for Registration{fn drop(&mut self){unsafe{let _=RevokeDragDrop(self.0);OleUninitialize();}}}
