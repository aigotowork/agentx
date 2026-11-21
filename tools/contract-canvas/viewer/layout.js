/* Presentation geometry only. Consumes only display groups, IDs and viewport dimensions. */
window.ContractLayout = (() => {
  const palette = ['blue','teal','violet','amber'];
  function overview(groups, narrow) {
    const columns=narrow?1:2, width=narrow?390:1080, groupWidth=narrow?358:510;
    let y=25;const nodes=[],boundaries=[];
    groups.forEach((group,index)=>{
      const column=index%columns, row=Math.floor(index/columns);
      const items=group.types.slice(0,narrow?4:4);const x=narrow?16:25+column*535;
      const gy=narrow?y:25+row*285;const h=265;
      boundaries.push({id:group.id,label:group.label,x,y:gy,w:groupWidth,h,color:palette[index%4],count:group.types.length});
      items.forEach((type,i)=>nodes.push({id:type.id,x:x+15+(narrow?0:(i%2)*242),y:gy+38+(narrow?i*112:Math.floor(i/2)*110),w:narrow?328:227,h:95}));
      if(narrow){boundaries[boundaries.length-1].h=50+items.length*112;y+=boundaries[boundaries.length-1].h+18;}
    });
    return {width,height:narrow?y:Math.ceil(groups.length/2)*285+20,nodes,boundaries};
  }
  function typeDetail(fieldCount,methodCount,neighbors,narrow) {
    const h=102+fieldCount*36+methodCount*37;
    const width=narrow?390:1060, x=narrow?25:315, y=50,w=narrow?340:410;
    const nodes=neighbors.slice(0,narrow?8:6).map((id,i)=>({id,x:narrow?25+(i%2)*180:(i%2?790:20),y:narrow?y+h+85+Math.floor(i/2)*110:75+Math.floor(i/2)*155,w:narrow?160:250,h:87}));
    return {width,height:narrow?y+h+100+Math.ceil(nodes.length/2)*110:Math.max(575,h+100),center:{x,y,w,h},nodes};
  }
  function methodTrace(inputCount,outputCount,narrow){
    const width=narrow?390:1080;
    const inputY=75, portY=narrow?inputY+inputCount*115+25:245;
    const port={x:narrow?35:390,y:portY,w:narrow?320:300,h:68};
    const inputs=Array.from({length:inputCount},(_,i)=>({x:narrow?45:35,y:narrow?inputY+i*115:70+i*140,w:narrow?300:280,h:86}));
    const outputs=Array.from({length:outputCount},(_,i)=>({x:narrow?45:770,y:narrow?portY+135+i*120:55+i*135,w:narrow?300:280,h:86}));
    return {width,height:narrow?portY+170+outputCount*120:Math.max(590,outputCount*135+90),port,inputs,outputs};
  }
  return {overview,typeDetail,methodTrace};
})();
/* Bounded Manhattan routing over this scene's rectangles, not a diagram editor. */
window.ContractRoutes = (() => {
  const clearance=12;
  const inside=(p,r)=>p.x>r.x-3&&p.x<r.x+r.w+3&&p.y>r.y-3&&p.y<r.y+r.h+3;
  function blocked(a,b,rects){
    return rects.some(r=>{
      const left=r.x-3,right=r.x+r.w+3,top=r.y-3,bottom=r.y+r.h+3;
      if(a.x===b.x)return a.x>left&&a.x<right&&Math.max(a.y,b.y)>top&&Math.min(a.y,b.y)<bottom;
      return a.y>top&&a.y<bottom&&Math.max(a.x,b.x)>left&&Math.min(a.x,b.x)<right;
    });
  }
  function route(a,b,rects,vertical){
    let start,end,first,last;
    if(vertical){
      const forward=b.y>=a.y+a.h;
      start={x:a.x+a.w/2,y:forward?a.y+a.h:a.y};end={x:b.x+b.w/2,y:forward?b.y:b.y+b.h};
      first={x:start.x,y:start.y+(forward?clearance:-clearance)};last={x:end.x,y:end.y+(forward?-clearance:clearance)};
    }else{
      const forward=b.x>=a.x+a.w;
      start={x:forward?a.x+a.w:a.x,y:a.y+a.h/2};end={x:forward?b.x:b.x+b.w,y:b.y+b.h/2};
      first={x:start.x+(forward?clearance:-clearance),y:start.y};last={x:end.x+(forward?-clearance:clearance),y:end.y};
    }
    // A method terminal can sit inside its owner's card; move the stub beyond that card.
    for(const r of rects){
      if(inside(first,r)){
        if(vertical)first.y=start.y<end.y?r.y+r.h+clearance:r.y-clearance;
        else first.x=start.x<end.x?r.x+r.w+clearance:r.x-clearance;
      }
      if(inside(last,r)){
        if(vertical)last.y=end.y>start.y?r.y-clearance:r.y+r.h+clearance;
        else last.x=end.x>start.x?r.x-clearance:r.x+r.w+clearance;
      }
    }
    const xs=[...new Set([first.x,last.x,...rects.flatMap(r=>[r.x-clearance,r.x+r.w+clearance])])].sort((x,y)=>x-y);
    const ys=[...new Set([first.y,last.y,...rects.flatMap(r=>[r.y-clearance,r.y+r.h+clearance])])].sort((x,y)=>x-y);
    const key=(x,y)=>x+','+y, distances=new Map(),previous=new Map();
    const startKey=key(xs.indexOf(first.x),ys.indexOf(first.y)),endKey=key(xs.indexOf(last.x),ys.indexOf(last.y));
    const open=[{key:startKey,x:xs.indexOf(first.x),y:ys.indexOf(first.y),distance:0}];distances.set(startKey,0);
    while(open.length){
      open.sort((a,b)=>b.distance-a.distance);const current=open.pop();if(current.key===endKey)break;
      if(current.distance!==distances.get(current.key))continue;
      const point={x:xs[current.x],y:ys[current.y]};
      for(const [dx,dy] of [[1,0],[-1,0],[0,1],[0,-1]]){
        const nx=current.x+dx,ny=current.y+dy;if(nx<0||ny<0||nx>=xs.length||ny>=ys.length)continue;
        const next={x:xs[nx],y:ys[ny]};if(blocked(point,next,rects))continue;
        const nextKey=key(nx,ny),distance=current.distance+Math.abs(next.x-point.x)+Math.abs(next.y-point.y);
        if(distance<(distances.get(nextKey)??Infinity)){distances.set(nextKey,distance);previous.set(nextKey,current.key);open.push({key:nextKey,x:nx,y:ny,distance});}
      }
    }
    const middle=[];
    if(distances.has(endKey)){
      let cursor=endKey;while(cursor){const [x,y]=cursor.split(',').map(Number);middle.push({x:xs[x],y:ys[y]});cursor=previous.get(cursor);}middle.reverse();
    }else {middle.push(first,{x:first.x,y:last.y},last);}
    const raw=[start,...middle,end],points=[];
    for(const p of raw){if(points.length&&p.x===points.at(-1).x&&p.y===points.at(-1).y)continue;while(points.length>1){const p0=points.at(-2),p1=points.at(-1);if((p0.x===p1.x&&p1.x===p.x)||(p0.y===p1.y&&p1.y===p.y))points.pop();else break;}points.push(p);}
    const d=points.map((p,i)=>(i?'L':'M')+p.x+','+p.y).join(' ');
    let longest={length:0,a:points[0],b:points[1]};for(let i=1;i<points.length;i++){const p=points[i-1],q=points[i],length=Math.abs(p.x-q.x)+Math.abs(p.y-q.y);if(length>longest.length)longest={length,a:p,b:q};}
    return {d,points,label:{x:(longest.a.x+longest.b.x)/2,y:(longest.a.y+longest.b.y)/2-6}};
  }
  return {route,blocked};
})();
