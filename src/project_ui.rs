//! Project panel geometry is shared by drawing and hit testing.
use crate::{project_catalog::{ProjectEntry,ProjectId},viewport_ui::{rect,text}};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ProjectAction {Toggle,SaveCurrent,SaveCopy,New,Open(ProjectId),Previous,Next,Close,Ignore}
#[derive(Default)]
pub struct ProjectPanel {pub open:bool,pub busy:bool,pub entries:Vec<ProjectEntry>,pub message:String,pub page:usize}
fn rows(height:u32)->usize {((height.saturating_sub(232))/44).clamp(1,10) as usize}
impl ProjectPanel {
    pub fn navigate(&mut self,action:ProjectAction,height:u32){
        match action {ProjectAction::Toggle=>{self.open=!self.open;self.page=0;},ProjectAction::Close=>self.open=false,
            ProjectAction::Previous=>self.page=self.page.saturating_sub(1),
            ProjectAction::Next=>{if (self.page+1)*rows(height)<self.entries.len(){self.page+=1;}},_=>{}}
    }
    pub fn hit(&self,width:u32,height:u32,x:f32,y:f32)->ProjectAction{
        if self.busy||!x.is_finite()||!y.is_finite()||x<8.0||x>=width as f32-8.0{return ProjectAction::Ignore;}
        if (104.0..148.0).contains(&y){
            let cell=(width as f32-16.0)/6.0;
            return match ((x-8.0)/cell) as usize {0=>ProjectAction::SaveCurrent,1=>ProjectAction::SaveCopy,2=>ProjectAction::New,3=>ProjectAction::Previous,4=>ProjectAction::Next,5=>ProjectAction::Close,_=>ProjectAction::Ignore};
        }
        if y>=176.0&&y<height as f32-56.0 {
            let row=((y-176.0)/44.0) as usize;
            if row<rows(height){if let Some(entry)=self.entries.get(self.page*rows(height)+row){return ProjectAction::Open(entry.id);}}
        }
        ProjectAction::Ignore
    }
    pub fn draw(&self,vertices:&mut Vec<f32>,width:u32,height:u32){
        if !self.open{return;}
        let w=width as f32;let h=height as f32;
        rect(vertices,w,h,0.0,68.0,w,(h-116.0).max(0.0),[0.035,0.045,0.065]);
        text(vertices,w,h,"PROJECTS - TAP ROW TO OPEN",16.0,92.0,1.7,[0.9,0.95,1.0]);
        let cell=(w-16.0)/6.0;
        for (i,label) in ["SAVE","COPY","NEW","PREV","NEXT","CLOSE"].iter().enumerate(){
            let x=8.0+i as f32*cell;
            rect(vertices,w,h,x,104.0,cell-4.0,44.0,if self.busy{[0.12,0.13,0.16]}else{[0.12,0.23,0.33]});
            let scale=((cell-12.0)/(label.len() as f32*4.0)).clamp(0.8,1.5);
            text(vertices,w,h,label,x+6.0,130.0,scale,[0.95,0.95,1.0]);
        }
        let max_chars=((w-32.0)/4.8).max(0.0) as usize;
        let message:String=if self.busy{"WORKING - PLEASE WAIT"}else{&self.message}.chars().take(max_chars).collect();
        text(vertices,w,h,&message,16.0,164.0,1.2,[1.0,0.78,0.4]);
        for (row,entry) in self.entries.iter().skip(self.page*rows(height)).take(rows(height)).enumerate(){
            let y=176.0+row as f32*44.0;
            rect(vertices,w,h,8.0,y,w-16.0,40.0,[0.08,0.11,0.16]);
            let name:String=entry.name.chars().take(((w-48.0)/6.0).max(0.0)as usize).collect();
            text(vertices,w,h,&name,20.0,y+26.0,1.5,[0.85,0.9,0.98]);
        }
        if self.entries.is_empty()&&!self.busy {text(vertices,w,h,"NO PROJECTS - USE SAVE COPY",16.0,200.0,1.4,[0.65,0.7,0.8]);}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn buttons_match_layout_at_phone_and_tablet_widths(){for w in [320,720,1280]{let p=ProjectPanel::default();let cell=(w as f32-16.0)/6.0;for(i,a)in [ProjectAction::SaveCurrent,ProjectAction::SaveCopy,ProjectAction::New,ProjectAction::Previous,ProjectAction::Next,ProjectAction::Close].into_iter().enumerate(){assert_eq!(p.hit(w,720,8.0+(i as f32+0.5)*cell,120.0),a);}}}
    #[test]fn busy_panel_blocks_actions_and_draws_finite_vertices(){let mut p=ProjectPanel::default();p.open=true;p.busy=true;assert_eq!(p.hit(360,640,30.0,120.0),ProjectAction::Ignore);let mut v=Vec::new();p.draw(&mut v,360,640);assert!(!v.is_empty());assert!(v.iter().all(|n|n.is_finite()));assert_eq!(v.len()%5,0);}
    #[test]fn empty_paging_is_bounded(){let mut p=ProjectPanel::default();p.navigate(ProjectAction::Previous,640);p.navigate(ProjectAction::Next,640);assert_eq!(p.page,0);assert_eq!(p.hit(360,640,30.0,190.0),ProjectAction::Ignore);}
    #[test]fn row_selection_tracks_page_and_cannot_select_hidden_rows(){
        let mut p=ProjectPanel::default();p.entries=(1..=30).map(|id|ProjectEntry{id:ProjectId(id),name:format!("PROJECT {id}")}).collect();
        assert_eq!(p.hit(360,640,30.0,190.0),ProjectAction::Open(ProjectId(1)));
        p.navigate(ProjectAction::Next,640);
        assert_eq!(p.hit(360,640,30.0,190.0),ProjectAction::Open(ProjectId(rows(640) as u64+1)));
        assert_eq!(p.hit(360,640,30.0,630.0),ProjectAction::Ignore);
        for _ in 0..20{p.navigate(ProjectAction::Next,640);}
        assert!(p.page*rows(640)<p.entries.len());
    }

}
