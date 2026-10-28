//! Toile de pixels (D-158) : `data-revel-pixels` sur un bloc ; le visiteur y dessine en
//! pixels (`draw`, les pixels restent) ou laisse une traînée qui s'efface (`trail`), au survol
//! ou en gardant le bouton appuyé (`click`). Comme le composant « Pixel Trail » de Framer, repris
//! à l'import.
//!
//! Options : `draw` ou `trail` (par défaut) ; `size=<px>` (côté d'un pixel, 20 par défaut),
//! `color=<couleur>` (noir par défaut ; une couleur du site `var(--nom)` suit le thème), `click` (dessin en appuyant) ; `fade=<s>` (durée de la
//! traînée, 1 s par défaut).

const SCRIPT: &str = concat!(
    "<script>(function(w,d){function opt(e){var o=(e.getAttribute(\"data-revel-pixels\")||\"\").split(/\\s+/),r={};",
    "o.forEach(function(x){var i=x.indexOf(\"=\");if(i>0)r[x.slice(0,i)]=x.slice(i+1);else if(x)r[x]=true});return r}",
    "function init(e){if(e.__rvPx)return;e.__rvPx=1;var o=opt(e),size=Math.max(2,parseFloat(o.size)||20),color=o.color||\"#000\",",
    "draw=!!o.draw,click=!!o.click,fade=(parseFloat(o.fade)||1)*1000,c=d.createElement(\"canvas\"),x=c.getContext(\"2d\");",
    "if(getComputedStyle(e).position===\"static\")e.style.position=\"relative\";",
    "c.setAttribute(\"aria-hidden\",\"true\");c.style.cssText=\"position:absolute;inset:0;width:100%;height:100%;pointer-events:none;image-rendering:pixelated\";e.appendChild(c);",
    "var cols=0,rows=0,cells={},down=false,last=null,raf=0;",
    "function size_(){var r=e.getBoundingClientRect();cols=Math.ceil(r.width/size);rows=Math.ceil(r.height/size);c.width=cols;c.height=rows;paint()}",
    "function paint(){raf=0;var now=Date.now(),more=false,fill=color,m=/^var\\((--[^,)]+)/.exec(color);",
    "if(m)fill=getComputedStyle(e).getPropertyValue(m[1]).trim()||\"#000\";x.clearRect(0,0,cols,rows);for(var k in cells){var t=cells[k],a=1;",
    "if(!draw){a=1-(now-t)/fade;if(a<=0){delete cells[k];continue}more=true}x.globalAlpha=a;x.fillStyle=fill;x.fillRect(k%cols,Math.floor(k/cols),1,1)}",
    "x.globalAlpha=1;if(more)raf=requestAnimationFrame(paint)}",
    "function cell(ev){var r=e.getBoundingClientRect(),cx=Math.floor((ev.clientX-r.left)/size),cy=Math.floor((ev.clientY-r.top)/size);",
    "return cx>=0&&cy>=0&&cx<cols&&cy<rows?[cx,cy]:null}",
    "function line(a,b){var x0=a[0],y0=a[1],dx=Math.abs(b[0]-x0),dy=Math.abs(b[1]-y0),sx=x0<b[0]?1:-1,sy=y0<b[1]?1:-1,err=dx-dy,now=Date.now();",
    "for(var i=0;i<500;i++){cells[y0*cols+x0]=now;if(x0===b[0]&&y0===b[1])break;var e2=2*err;if(e2>-dy){err-=dy;x0+=sx}if(e2<dx){err+=dx;y0+=sy}}}",
    "function move(ev){if(click&&!down)return;var p=cell(ev);if(!p){last=null;return}line(last||p,p);last=p;if(!raf)raf=requestAnimationFrame(paint)}",
    "e.addEventListener(\"pointermove\",move);e.addEventListener(\"pointerleave\",function(){last=null});",
    "e.addEventListener(\"pointerdown\",function(ev){down=true;last=null;move(ev)});w.addEventListener(\"pointerup\",function(){down=false;last=null});",
    "if(w.ResizeObserver)new ResizeObserver(size_).observe(e);size_()}",
    "function all(){[].forEach.call(d.querySelectorAll(\"[data-revel-pixels]\"),init)}",
    "if(d.readyState===\"loading\")d.addEventListener(\"DOMContentLoaded\",all);else all()})(window,document)</script>",
);

/// La page contient une toile de pixels.
pub fn uses(html: &str) -> bool {
    html.contains(" data-revel-pixels")
}

/// Page (site, aperçu ou canevas) : script ajouté avant `</head>` si elle a une toile.
pub fn wire(html: String) -> String {
    if uses(&html) {
        crate::insert_before(&html, "</head>", SCRIPT, false)
    } else {
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toile_de_pixels() {
        let page = "<html><head></head><body><div data-revel-pixels=\"draw size=7 click\"></div></body></html>".to_string();
        let out = wire(page.clone());
        assert!(
            out.contains("getContext")
                && out.find("<script>").unwrap() < out.find("</head>").unwrap()
        );
    }
}
