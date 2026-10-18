import {contentSecurityPolicy} from '../csp';
it('production assets stay external and the preview applies the native content policy',async()=>{
 const {default:config}=await import('../vite.config');
 expect(config.build?.assetsInlineLimit).toBe(0);
 expect(config.preview?.headers?.['Content-Security-Policy']).toBe(contentSecurityPolicy);
 expect(config.preview?.host).toBe('127.0.0.1');
 expect(config.server?.strictPort).toBe(true);
});
