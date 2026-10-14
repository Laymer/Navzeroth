import { expect, test } from "@playwright/test";
for (const width of [320, 640]) {
  for (const fontSize of [16, 32]) {
    test(`primary navigation fits ${width}px with ${fontSize}px root text and retains keyboard access`, async ({ page }) => {
      await page.setViewportSize({width,height:900});
      await page.goto("/");
      await page.addStyleTag({content:`:root { font-size: ${fontSize}px; }`});
      const nav=page.getByRole("navigation",{name:"Primary"});
      const buttons=nav.getByRole("button");
      await expect(buttons).toHaveCount(13);
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
