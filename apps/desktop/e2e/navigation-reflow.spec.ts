import { expect, test } from "@playwright/test";
for (const width of [320, 640]) {
  for (const fontSize of [16, 32]) {
    test(`primary navigation fits ${width}px with ${fontSize}px root text and retains keyboard access`, async ({ page }) => {
      await page.setViewportSize({width,height:900});
      await page.goto("/");
      await page.addStyleTag({content:`:root { font-size: ${fontSize}px; }`});
      const nav=page.getByRole("navigation",{name:"Primary"});
      const buttons=nav.getByRole("button");
      await expect(buttons).toHaveCount(14);
      const geometry=await nav.evaluate(el=>({width:el.clientWidth,scrollWidth:el.scrollWidth,overflow:getComputedStyle(el).overflowX,buttons:Array.from(el.querySelectorAll("button")).map(button=>{const r=button.getBoundingClientRect();return {text:button.textContent,left:r.left,right:r.right,width:r.width};})}));
      expect(geometry.overflow).toBe("visible");
      expect(geometry.scrollWidth).toBeLessThanOrEqual(geometry.width);
      for(const button of geometry.buttons) { expect(button.left,button.text??"button").toBeGreaterThanOrEqual(0); expect(button.right,button.text??"button").toBeLessThanOrEqual(width); expect(button.width).toBeGreaterThan(0); }
      await expect(nav.getByRole("button",{name:"Today",exact:true})).toHaveAttribute("aria-current","page");
      await buttons.first().focus();
      for(let index=0;index<await buttons.count();index++) { await expect(buttons.nth(index)).toBeFocused(); if(index+1<await buttons.count()) await page.keyboard.press("Tab"); }
      const imports=nav.getByRole("button",{name:"Imports",exact:true});
      await imports.focus(); await imports.press("Enter");
      await expect(imports).toBeFocused(); await expect(imports).toHaveAttribute("aria-current","page");
      await expect(page.getByRole("heading",{name:"Snapshot imports",exact:true})).toBeVisible();
    });
  }
}

// G4 review #4: the rail/bar switch follows the Settings text size, not the browser's fixed 16 px.
for (const [fontSize, layout] of [[16, "rail"], [32, "bar"]] as const) {
  test(`a 1100px window shows the ${layout} at ${fontSize}px root text`, async ({ page }) => {
    await page.setViewportSize({ width: 1100, height: 900 });
    await page.goto("/");
    await page.evaluate((size) => { document.documentElement.style.fontSize = String(size) + "px"; }, fontSize);
    const nav = page.getByRole("navigation", { name: "Primary" });
    await expect(nav.getByRole("button")).toHaveCount(14);
    const boxes = await page.evaluate(() => {
      const n = document.querySelector(".app-nav")?.getBoundingClientRect();
      const c = document.querySelector(".app-content")?.getBoundingClientRect();
      return { navRight: n?.right ?? 0, navBottom: n?.bottom ?? 0, contentLeft: c?.left ?? 0, contentTop: c?.top ?? 0 };
    });
    if (layout === "rail") expect(boxes.contentLeft).toBeGreaterThanOrEqual(boxes.navRight);
    else expect(boxes.contentTop).toBeGreaterThanOrEqual(boxes.navBottom);
    const widths = await page.evaluate(() => ({ scroll: document.documentElement.scrollWidth, client: document.documentElement.clientWidth }));
    expect(widths.scroll).toBeLessThanOrEqual(widths.client);
  });
}
