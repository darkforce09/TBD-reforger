//! Catalog selection, inline expansion and history preserve the reading context.
use super::super::cdp;
use anyhow::{Result, ensure};
use serde_json::Value;
use std::path::Path;
pub async fn verify(page: &cdp::Page, website: &str, output: &Path) -> Result<Value> {
    page.navigate(&format!("{website}/debug/data-viewer"))
        .await?;
    ensure!(
        page.wait_for(
            "document.querySelector('.dv-capabilities a[href*=attachment]')!==null",
            100,
            150
        )
        .await?,
        "attachment entry missing"
    );
    page.evaluate(
        "document.querySelector('.dv-capabilities a[href*=attachment]').click()",
        false,
    )
    .await?;
    ensure!(
        page.wait_for(
            "document.querySelector('.dv-resource-chooser .dv-list-row')!==null",
            100,
            150
        )
        .await?,
        "attachment list missing"
    );
    page.evaluate("(()=>{const form=document.querySelector('.dv-resource-chooser form');const input=form.querySelector('input');input.value='scope';input.dispatchEvent(new Event('input',{bubbles:true}));form.requestSubmit();})()",false).await?;
    ensure!(page.wait_for("location.search.includes('q=scope') && document.querySelector('.dv-resource-chooser .dv-list-row')?.innerText.toLowerCase().includes('scope')",100,150).await?,"scope search failed");
    page.evaluate("(()=>{const list=document.querySelector('.dv-resource-chooser .dv-virtual');list.scrollTop=144;list.dispatchEvent(new Event('scroll'));window.__gridCatalog={list,scroll:list.scrollTop};})()",false).await?;
    page.evaluate(
        "document.querySelector('.dv-resource-chooser .dv-list-row').click()",
        false,
    )
    .await?;
    ensure!(
        page.wait_for(
            "document.querySelectorAll('.dv-field-toggle').length>4",
            100,
            150
        )
        .await?,
        "resource cards did not load"
    );
    ensure!(page.evaluate("new URLSearchParams(location.search).get('catalog_capability')==='attachment' && new URLSearchParams(location.search).get('q')==='scope' && window.__gridCatalog.list===document.querySelector('.dv-resource-chooser .dv-virtual') && window.__gridCatalog.scroll===window.__gridCatalog.list.scrollTop",false).await?==true,"resource selection reset the catalog");
    page.evaluate("(()=>{const fields=document.querySelectorAll('.dv-field-toggle');fields[0].click();fields[1].click();window.__gridFirst=location.href;})()",false).await?;
    ensure!(
        page.wait_for(
            "document.querySelectorAll('.dv-inline-details').length>=2",
            100,
            100
        )
        .await?,
        "multiple fields did not expand"
    );
    std::fs::write(
        output.join("screenshots/scope-card-grid.png"),
        page.screenshot().await?,
    )?;
    page.evaluate("(()=>{const grid=document.querySelector('.dv-card-scroll');grid.scrollTop=180;grid.dispatchEvent(new Event('scroll'));window.__gridScroll=grid.scrollTop;const next=[...document.querySelectorAll('.dv-resource-chooser .dv-list-row')].find(a=>new URL(a.href).searchParams.get('resource')!==new URL(location.href).searchParams.get('resource'));next.click();})()",false).await?;
    ensure!(page.wait_for("location.href!==window.__gridFirst && document.querySelector('.dv-field-toggle')!==null",100,150).await?,"second resource failed");
    page.evaluate("history.back()", false).await?;
    ensure!(page.wait_for("location.href===window.__gridFirst && document.querySelectorAll('.dv-inline-details').length>=2",100,150).await?,"Back lost expanded values");
    ensure!(page.evaluate("Math.abs(document.querySelector('.dv-card-scroll').scrollTop-window.__gridScroll)<3 && window.__gridCatalog.list.scrollTop===window.__gridCatalog.scroll",false).await?==true,"Back lost reading position");
    let result=page.evaluate("({catalog_filter:new URLSearchParams(location.search).get('catalog_capability'),expanded:document.querySelectorAll('.dv-inline-details').length,mounted_cards:document.querySelectorAll('.dv-component-card').length,nested_scrollbars:[...document.querySelectorAll('.dv-component-card *')].filter(e=>['auto','scroll'].includes(getComputedStyle(e).overflowY)&&e.scrollHeight>e.clientHeight).length})",false).await?;
    ensure!(
        result["nested_scrollbars"] == 0,
        "cards contain nested vertical scrolling: {result}"
    );
    std::fs::write(
        output.join("grid-navigation.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    Ok(result)
}

pub async fn large_vehicle(page: &cdp::Page, website: &str, output: &Path) -> Result<()> {
    page.set_viewport(1600, 1000).await?;
    page.navigate(&format!(
        "{website}/debug/data-viewer?tab=resources&resource=guid%3ABD75A18920BD1278&view=all"
    ))
    .await?;
    ensure!(
        page.wait_for(
            "document.querySelectorAll('.dv-field-toggle').length>10",
            100,
            150
        )
        .await?,
        "large vehicle did not render"
    );
    page.evaluate("(()=>{const original=window.fetch;window.__cardFailure=false;window.fetch=async function(...args){const u=typeof args[0]==='string'?args[0]:args[0].url;if(!window.__cardFailure&&u.includes('/resource-cards?')&&u.includes('cursor=')){window.__cardFailure=true;return new Response(JSON.stringify({error:'Simulated card failure'}),{status:500,headers:{'Content-Type':'application/json'}});}return original.apply(this,args);};const grid=document.querySelector('.dv-card-scroll');grid.scrollTop=grid.scrollHeight*.75;grid.dispatchEvent(new Event('scroll'));})()",false).await?;
    ensure!(page.wait_for("[...document.querySelectorAll('.dv-component-card p')].some(p=>p.textContent.includes('Simulated card failure'))",100,150).await?,"card failure did not stay local");
    page.evaluate("[...document.querySelectorAll('.dv-component-card button')].find(b=>b.textContent==='Retry').click()",false).await?;
    ensure!(page.wait_for("document.querySelectorAll('.dv-field-toggle').length>10 && ![...document.querySelectorAll('.dv-component-card p')].some(p=>p.textContent.includes('Simulated card failure'))",100,150).await?,"card retry failed");
    let result=page.evaluate("({mounted_cards:document.querySelectorAll('.dv-component-card').length,first_card:Number(document.querySelector('.dv-component-card').dataset.cardIndex),api_responses:performance.getEntriesByType('resource').filter(r=>r.name.includes('/debug/equipment-data/')).map(r=>r.decodedBodySize),failure_recovered:window.__cardFailure})",false).await?;
    ensure!(
        result["mounted_cards"].as_u64().unwrap_or(999) <= 18,
        "large grid mounts too many cards: {result}"
    );
    ensure!(
        result["first_card"].as_u64().unwrap_or(0) > 100,
        "large grid did not navigate beyond initial cards: {result}"
    );
    std::fs::write(
        output.join("large-grid.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    std::fs::write(
        output.join("screenshots/large-vehicle-grid.png"),
        page.screenshot().await?,
    )?;
    Ok(())
}
