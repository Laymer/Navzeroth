import {act,fireEvent,render,screen} from "@testing-library/react";
import {MapView,type MapViewProps} from "./MapView";
import type {AtlasView} from "../commands/contract";
import fixture from "../../../../fixtures/desktop/atlas/query-atlas.json";
const image=(fixture as unknown as AtlasView).images[0];if(!image)throw Error("Missing atlas fixture");
const props:MapViewProps={image,layer:null,onLayerError:vi.fn(),marker:{image:image.id,pixel:{x:100,y:100},approximate:true},markerStale:true,selectedPin:null,focus:null};
it("supports bounded wheel, keyboard and pointer navigation with resize cleanup",()=>{
 let width=800;let measure:()=>void=()=>{throw Error("Observer not installed");};const disconnect=vi.fn();
 vi.stubGlobal("ResizeObserver",class{constructor(callback:()=>void){measure=callback;}observe(){}disconnect=disconnect;});
 // jsdom lacks PointerEvent on older versions; exercise the browser coordinates
 // through MouseEvent's shared coordinate/button fields.
 vi.stubGlobal("PointerEvent",MouseEvent);
 const rect=vi.spyOn(HTMLElement.prototype,"getBoundingClientRect").mockImplementation(()=>({x:0,y:0,left:0,top:0,right:width,bottom:600,width,height:600,toJSON(){return {};}}));
 const view=render(<MapView {...props}/>);
 try{
  const region=screen.getByRole("group",{name:`Atlas map of ${image.id}`});
  const frame=()=>{const node=view.container.querySelector("rect.frame");if(!node)throw Error("Frame missing");return {width:Number(node.getAttribute("width")),x:Number(node.getAttribute("x")),y:Number(node.getAttribute("y"))};};
  const fitted=frame();expect(screen.getByRole("img")).toHaveAttribute("width","800");
  fireEvent.click(screen.getByRole("button",{name:"Zoom in"}));expect(frame().width).toBeGreaterThan(fitted.width);
  fireEvent.click(screen.getByRole("button",{name:"Zoom out"}));expect(frame().width).toBeCloseTo(fitted.width);
  fireEvent.wheel(region,{deltaY:-120,clientX:200,clientY:200});const zoomed=frame();expect(zoomed.width).toBeGreaterThan(fitted.width);
  fireEvent.keyDown(region,{key:"ArrowDown",ctrlKey:true});expect(frame()).toEqual(zoomed);
  fireEvent.keyDown(region,{key:"Tab"});expect(frame()).toEqual(zoomed);
  fireEvent.keyDown(region,{key:"ArrowDown"});expect(frame()).not.toEqual(zoomed);
  fireEvent.pointerMove(region,{clientX:250,clientY:250});const idle=frame();
  fireEvent.pointerDown(region,{button:2,clientX:250,clientY:250});fireEvent.pointerMove(region,{clientX:280,clientY:280});expect(frame()).toEqual(idle);
  fireEvent.pointerDown(region,{button:0,clientX:250,clientY:250});fireEvent.pointerMove(region,{clientX:280,clientY:280});expect(frame()).not.toEqual(idle);
  fireEvent.pointerUp(region);const stopped=frame();fireEvent.pointerMove(region,{clientX:300,clientY:300});expect(frame()).toEqual(stopped);
  fireEvent.pointerDown(region,{button:0,clientX:300,clientY:300});fireEvent.pointerCancel(region);fireEvent.pointerMove(region,{clientX:320,clientY:320});expect(frame()).toEqual(stopped);
  fireEvent.click(screen.getByRole("button",{name:"Centre on marker"}));expect(frame()).not.toEqual(stopped);
  fireEvent.click(screen.getByRole("button",{name:"Fit to frame"}));expect(frame().width).toBeCloseTo(fitted.width);
  width=500;act(()=>measure());expect(screen.getByRole("img")).toHaveAttribute("width","500");
  expect(frame().width).toBeLessThanOrEqual(500);
  view.unmount();expect(disconnect).toHaveBeenCalledOnce();
 }finally{view.unmount();rect.mockRestore();vi.unstubAllGlobals();}
});
