function fibRec(n){ if(n<=1) return n; return fibRec(n-1)+fibRec(n-2); }
function fibIter(n){ let a=0,b=1; for(let i=0;i<n;i++){ let t=a; a=b; b=t+b; } return a; }
function dot(v1,v2){ return v1.reduce((s,x,i)=>s+x*v2[i],0); }
function norm(v){ return Math.sqrt(v.reduce((s,x)=>s+x*x,0)); }
function softmax(v){ let e=v.map(Math.exp); let s=e.reduce((a,b)=>a+b,0); return e.map(x=>x/s); }

let v1=[1,2,3,4,5], v2=[5,4,3,2,1];

let start=Date.now(); let r1=fibRec(25); let t1=Date.now()-start;
start=Date.now(); let r2=fibIter(35); let t2=Date.now()-start;
start=Date.now(); let dot_val=dot(v1,v2); let norm_val=norm(v1); let sm=softmax(v1); let t3=Date.now()-start;

console.log(`fibRec(25) = ${r1} | tempo = ${t1} ms`);
console.log(`fibIter(35) = ${r2} | tempo = ${t2} ms`);
console.log(`dot(v1,v2) = ${dot_val}`);
console.log(`norm(v1) = ${norm_val}`);
console.log(`softmax(v1) = ${sm} | tempo = ${t3} ms`);