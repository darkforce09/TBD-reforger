//! Live acceptance checks use the normal website and its imported generation.
use super::cdp;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::Path;
mod grid_navigation;
mod polling;

pub async fn run(website: &str, output: &Path) -> Result<u8> {
    std::fs::create_dir_all(output.join("screenshots"))?;
    let browser = cdp::launch(9358, &[]).await?;
    let result = verify(&browser, website, output).await;
    browser.shutdown().await;
    result?;
    println!("PASS: equipment viewer through {website}/debug/data-viewer");
    Ok(0)
}

async fn verify(browser: &cdp::Browser, website: &str, output: &Path) -> Result<()> {
    let page = cdp::new_page(browser, None, &[]).await?;
    let api = format!("{website}/api/v1/debug/equipment-data");
    let client = reqwest::Client::new();
    let status: Value = client
        .get(format!("{api}/status"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let generation = status["generation_id"]
        .as_str()
        .context("no imported generation")?;
    let started = std::time::Instant::now();
    page.navigate(&format!("{website}/debug/data-viewer"))
        .await?;
    ensure!(
        page.wait_for("document.querySelectorAll('.dv-card').length===3", 100, 300)
            .await?,
        "overview did not render"
    );
    let overview_ms = started.elapsed().as_millis();
    let bytes=page.evaluate("performance.getEntriesByType('resource').filter(r=>r.name.includes('/debug/equipment-data/')).reduce((n,r)=>n+r.decodedBodySize,0)",false).await?;
    ensure!(
        bytes.as_u64().unwrap_or(u64::MAX) <= 1048576,
        "overview exceeds 1 MiB"
    );
    ensure!(page.evaluate("!document.querySelector('aside[aria-label=\"Main navigation\"]') && !localStorage.getItem('access_token')",false).await?==true,"anonymous/full-window assertion failed");
    std::fs::write(
        output.join("screenshots/overview.png"),
        page.screenshot().await?,
    )?;
    let polling = polling::verify(&page).await?;
    std::fs::write(
        output.join("polling-stability.json"),
        serde_json::to_vec_pretty(&polling)?,
    )?;
    let mut resources = Vec::new();
    for search in [
        "Backpack_IIFS_FieldPack.et",
        "M997_maxi_ambulance.et",
        "Rifle_M16A2_M203.et",
        "Mortar",
        "Radio",
        "Morphine",
        "Saline",
        "Binocular",
        "Grenade",
        "Mi8",
    ] {
        let result: Value = client
            .get(with_query(
                &format!("{api}/resources"),
                &[("generation", generation), ("q", search)],
            )?)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let resource = result["items"]
            .as_array()
            .and_then(|a| a.first())
            .context(format!("acceptance resource absent: {search}"))?;
        resources.push(resource.clone());
    }
    let pack = &resources[0];
    let mut location = reqwest::Url::parse(&format!("{website}/debug/data-viewer"))?;
    location.query_pairs_mut().extend_pairs([
        ("tab", "resources"),
        ("resource", pack["resource_id"].as_str().unwrap()),
        ("capability", "storage"),
    ]);
    page.navigate(location.as_str()).await?;
    ensure!(page.wait_for("document.body.innerText.includes('IIFS Large Combat Field Pack') && document.querySelectorAll('.dv-list-row').length>0",100,200).await?,"resource inspector did not render");
    let containers: Value = client
        .get(with_query(
            &format!("{api}/containers"),
            &[
                ("resource_id", pack["resource_id"].as_str().unwrap()),
                ("capability", "storage"),
                ("generation", generation),
            ],
        )?)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let node = containers["items"][0]["node_id"]
        .as_str()
        .context("pack storage missing")?;
    location
        .query_pairs_mut()
        .extend_pairs([("node", node), ("property", "MaxCumulativeVolume")]);
    page.navigate(location.as_str()).await?;
    ensure!(
        page.wait_for(
            "document.querySelector('.dv-inline-details')?.innerText.includes('15000')",
            100,
            200
        )
        .await?,
        "source value did not render"
    );
    ensure!(
        page.evaluate(
            "document.body.innerText.includes('Unit unspecified')",
            false
        )
        .await?
            == true,
        "unknown units hidden"
    );
    ensure!(
        page.evaluate(
            "document.querySelector('.dv-inline-details')?.innerText.includes('15000')",
            false
        )
        .await?
            == true,
        "inline field value missing"
    );
    std::fs::write(
        output.join("screenshots/iifs-storage.png"),
        page.screenshot().await?,
    )?;
    let inspector_polling = polling::verify(&page).await?;
    std::fs::write(
        output.join("inspector-polling-stability.json"),
        serde_json::to_vec_pretty(&inspector_polling)?,
    )?;
    page.evaluate(
        "document.querySelector('.dv-inline-details summary').click()",
        false,
    )
    .await?;
    page.evaluate(
        "document.querySelector('.dv-tabs a[href*=fields]').click()",
        false,
    )
    .await?;
    ensure!(
        page.wait_for(
            "document.querySelector('.dv-fields .dv-list-row')!==null",
            100,
            200
        )
        .await?,
        "field inventory did not render"
    );
    let mounted = page
        .evaluate(
            "document.querySelectorAll('.dv-fields .dv-list-row').length",
            false,
        )
        .await?;
    ensure!(
        mounted.as_u64().unwrap_or(100) <= 32,
        "field list is not virtualized"
    );
    std::fs::write(
        output.join("screenshots/field-inventory.png"),
        page.screenshot().await?,
    )?;
    page.evaluate(
        "document.querySelector('.dv-fields .dv-list-row').click()",
        false,
    )
    .await?;
    ensure!(page.wait_for("location.search.includes('field=') && document.querySelector('.dv-fields .dv-list-row')!==null",100,200).await?,"field occurrences did not render");
    page.evaluate(
        "document.querySelector('.dv-fields .dv-list-row').click()",
        false,
    )
    .await?;
    ensure!(
        page.wait_for(
            "document.querySelector('.dv-inline-details')!==null",
            100,
            200
        )
        .await?,
        "occurrence did not jump to its property"
    );
    page.evaluate("history.back()", false).await?;
    ensure!(
        page.wait_for("document.querySelector('.dv-fields')!==null", 100, 200)
            .await?,
        "Back lost field context"
    );
    page.set_viewport(760, 900).await?;
    page.navigate(location.as_str()).await?;
    ensure!(
        page.wait_for(
            "document.querySelector('.dv-inline-details')!==null",
            100,
            200
        )
        .await?,
        "narrow inspector did not render"
    );
    ensure!(
        page.evaluate("document.documentElement.scrollWidth<=innerWidth", false)
            .await?
            == true,
        "narrow page overflows horizontally"
    );
    std::fs::write(
        output.join("screenshots/narrow-inspector.png"),
        page.screenshot().await?,
    )?;
    grid_navigation::verify(&page, website, output).await?;
    grid_navigation::large_vehicle(&page, website, output).await?;
    let performance = json!({"generation_id":generation,"overview_ms":overview_ms,"overview_api_bytes":bytes,"mounted_field_rows":mounted,"normal_website":website,"verified_at":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs()});
    std::fs::write(
        output.join("performance.json"),
        serde_json::to_vec_pretty(&performance)?,
    )?;
    let coverage = json!({"generation_id":generation,"counts":status["overview"],"resources_inspected":resources,"checks":["anonymous overview","full-window route","paged resource lookup","source value and unspecified units","virtualized field list","field occurrence source jump","browser Back","narrow layout"]});
    std::fs::write(
        output.join("coverage.json"),
        serde_json::to_vec_pretty(&coverage)?,
    )?;
    Ok(())
}

fn with_query(url: &str, pairs: &[(&str, &str)]) -> Result<reqwest::Url> {
    let mut url = reqwest::Url::parse(url)?;
    url.query_pairs_mut().extend_pairs(pairs.iter().copied());
    Ok(url)
}
