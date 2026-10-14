/** Only the main desktop mounts this listener; native also checks actual focus. */
export function referenceShortcut(event:KeyboardEvent,enabled:boolean,busy:boolean,focused:boolean):boolean {
 if(!enabled||busy||!focused||event.defaultPrevented||event.repeat||event.isComposing||event.key!=="F8"||!event.ctrlKey||!event.shiftKey||event.altKey||event.metaKey)return false;
 const target=event.target;
 return !(target instanceof Element&&target.closest('textarea,select,input:not([type="checkbox"]):not([type="radio"]):not([type="button"]):not([type="submit"]):not([type="reset"]),[contenteditable]:not([contenteditable="false"]),[role="textbox"]'));
}
