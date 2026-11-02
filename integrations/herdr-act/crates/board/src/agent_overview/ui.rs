//! Draw responsive session families, concrete decisions, and the ACT workflow timeline.
use super::*;

impl App {
    pub(super) fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let rows = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2),Constraint::Min(4),Constraint::Length(1)]).split(area);
        let header = format!(" Agent Overview · {}{} · [p] project [Enter] open",
            self.project.as_deref().unwrap_or("All"), if self.show_hidden {" + hidden"} else {""});
        frame.render_widget(Paragraph::new(header).style(Style::default().add_modifier(Modifier::BOLD)),rows[0]);
        if self.attached_pane.is_some() {
            self.draw_list(frame,rows[1]);
        } else {
            let columns = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(42),Constraint::Percentage(58)]).split(rows[1]);
            self.draw_list(frame,columns[0]);
            self.draw_detail(frame,columns[1]);
        }
        let footer = self.error.as_deref().or(self.codex_error.as_deref())
            .unwrap_or("[1–5] groups [Space] children [w] workflow [PgUp/Dn] details [e] edit [m] done [h] hide [a] hidden [q] close");
        frame.render_widget(Paragraph::new(footer).style(Style::default().fg(Color::Yellow)),rows[2]);
    }
    fn draw_list(&self, frame: &mut Frame, area: Rect) {
        let rows = self.rows();
        let visible = self.visible();
        let mut lines = Vec::new();
        let mut selected_line = 0;
        for group in [OverviewGroup::NeedYou,OverviewGroup::Problems,OverviewGroup::Active,OverviewGroup::Done,OverviewGroup::Unknown] {
            let entries = rows.iter().filter(|r| r.group == group).collect::<Vec<_>>();
            if entries.is_empty() {continue;}
            let folded = self.collapsed_groups.contains(&group_order(group));
            let count = entries.iter().filter(|r|r.depth==0).count();
            lines.push(Line::from(Span::styled(format!(" {} {} ({count})",if folded {"▸"} else {"▾"},group_label(group)),Style::default().add_modifier(Modifier::BOLD))));
            for row in entries {
                if !visible.iter().any(|s|s.id==row.session.id) {continue;}
                let session=row.session;
                let selected=self.sessions.get(self.selected).is_some_and(|s|s.id==session.id);
                if selected {selected_line=lines.len();}
                let style=if selected {Style::default().bg(Color::Blue).fg(Color::White)} else {Style::default()};
                let branch=if row.depth>0 {format!("{}↳ ","  ".repeat(row.depth.min(6)))} else {String::new()};
                let phase=crate::store::titles::short_title(session.act_phase.as_deref().unwrap_or("Unbekannt"),14);
                let runtime=crate::store::titles::short_title(runtime_label(&session.runtime_status),12);
                let tag=format!("[{}]",if session.program.is_empty() {"?"} else {&session.program});
                let prefix=format!(" {} {branch}",if selected {"›"} else {" "});
                let inner=area.width.saturating_sub(2) as usize;
                if area.width<100 {
                    let budget=inner.saturating_sub(crate::text::width(&prefix)+crate::text::width(&tag)+1);
                    let title=crate::store::titles::short_title(&session.title,budget);
                    lines.push(Line::from(Span::styled(format!("{prefix}{title} {tag}"),style)));
                    if area.width<44 {
                        lines.push(Line::from(Span::styled(format!("   {}",crate::store::titles::short_title(&session.project,inner.saturating_sub(3))),style)));
                        lines.push(Line::from(Span::styled(format!("   {runtime} · {phase}"),style)));
                    } else {
                        let budget=inner.saturating_sub(9+crate::text::width(&runtime)+crate::text::width(&phase)).min(18);
                        let project=crate::store::titles::short_title(&session.project,budget);
                        lines.push(Line::from(Span::styled(format!("   {project} · {runtime} · {phase}"),style)));
                    }
                } else {
                    let project=crate::store::titles::short_title(&session.project,24);
                    let tail=format!(" {tag} · {project} · {runtime} · {phase}");
                    let title=crate::store::titles::short_title(&session.title,inner.saturating_sub(crate::text::width(&prefix)+crate::text::width(&tail)));
                    lines.push(Line::from(Span::styled(format!("{prefix}{title}{tail}"),style)));
                }
                if let Some(question)=&session.question {
                    lines.push(Line::from(Span::styled(format!("   ? {}",question.lines().next().unwrap_or("")),Style::default().fg(Color::Yellow))));
                }
            }
        }
        if lines.is_empty() {lines.push(Line::from(" No sessions in this project"));}
        let offset=selected_line.saturating_sub(area.height.saturating_sub(6) as usize);
        frame.render_widget(Paragraph::new(Text::from(lines)).scroll((offset.min(u16::MAX as usize) as u16,0))
            .block(Block::default().title(" Sessions ").borders(Borders::ALL)),area);
    }
    fn draw_detail(&self,frame:&mut Frame,area:Rect) {
        let Some(session)=self.current() else {
            frame.render_widget(Paragraph::new("No visible session selected").block(Block::default().title(" Selected session ").borders(Borders::ALL)),area);
            return;
        };
        let source=match &session.source {SessionSource::Herdr{..}=>"Herdr CLI",SessionSource::Native{..}=>"Herdr App",SessionSource::CodexApp{..}=>"Codex App",SessionSource::Ended=>"Beendete Sitzung"};
        let completion=match session.manual_completion {
            Some(true)=>"Done (manual)",Some(false)=>"Not done (manual)",
            None=>match session.task_complete {Some(true)=>"Done (explicit report)",Some(false)=>"Not done",None=>"Unknown"},
        };
        let phase=session.act_phase.as_deref().unwrap_or("Unbekannt");
        let mut lines=vec![
            Line::from(Span::styled(&session.title,Style::default().add_modifier(Modifier::BOLD))),
            Line::from(format!("Project: {} · {source}/{}",session.project,session.program)),
            Line::from(format!("Runtime: {} · Work: {completion}",runtime_label(&session.runtime_status))),
            Line::from(format!("ACT: {phase} · {}",session.phase_source.as_deref().unwrap_or("Unknown"))),
            Line::from(format!("Spec/Ticket: {}",session.ticket.as_deref().unwrap_or("Unknown"))),
        ];
        if let Some(question)=&session.question {
            lines.push(Line::from(Span::styled(format!("Braucht dich: {question}"),Style::default().fg(Color::Yellow))));
        } else if session.group==OverviewGroup::NeedYou {lines.push(Line::from("Waiting for user input; exact request has not been observed yet."));}
        if let Some(problem)=&session.problem {lines.push(Line::from(Span::styled(problem,Style::default().fg(Color::Red))));}
        lines.push(Line::from(format!("{} ACT-Verlauf ({}) [w]",if self.expanded_history.contains(&session.id) {"▾"} else {"▸"},session.phase_history.len())));
        for phase in session.phase_history.iter().rev().filter(|_|self.expanded_history.contains(&session.id)) {
            let source=if phase.source.is_empty() {"manual"} else {phase.source.as_str()};
            lines.push(Line::from(format!("ACT: {} · {} · {}{}",phase.phase,phase.command.as_deref().unwrap_or("manual"),source,
                if phase.ended_at.is_some() {" · finished"} else {""})));
        }
        lines.push(Line::from(""));
        if !self.terminal_text.is_empty() {lines.extend(self.terminal_text.lines().map(Line::from));}
        else if !session.preview.is_empty() {lines.push(Line::from(session.preview.as_str()));}
        frame.render_widget(Paragraph::new(Text::from(lines)).wrap(ratatui::widgets::Wrap{trim:true})
            .scroll((self.detail_scroll,0))
            .block(Block::default().title(" Selected session ").borders(Borders::ALL)),area);
    }
}
pub(super) fn group_order(group:OverviewGroup)->u8 {overview_tree::rank(group)}
fn runtime_label(status:&str)->&str {
    match status {"working"|"active"=>"Arbeitet","waiting-parent"=>"Wartet auf Hauptagent","idle"|"done"=>"Inaktiv",
        "waiting"=>"Wartet auf Eingabe","closed"=>"Beendet","interrupted"=>"Unterbrochen","notLoaded"|""|"unknown"=>"Unbekannt",other=>other}
}
fn group_label(group:OverviewGroup)->&'static str {
    match group {OverviewGroup::NeedYou=>"BRAUCHT DICH",OverviewGroup::Problems=>"PROBLEME",OverviewGroup::Active=>"AKTIV",OverviewGroup::Done=>"FERTIG",OverviewGroup::Unknown=>"UNBEKANNT"}
}
