//! CSS and JS strings injected into animated SVGs.

/// CSS appended into the `<style>` block of every animated SVG.
pub(super) const ANIMATION_CSS: &str = r##"
    /* Animation frames */
    .forge-frame { opacity: 0; pointer-events: none; transition: opacity 0.4s ease-in-out; }
    .forge-frame--active { opacity: 1; pointer-events: auto; }

    /* Pulse effect */
    @keyframes forge-pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }
    .forge-pulse { animation: forge-pulse 1.5s ease-in-out infinite; }

    /* Fade-in for new elements */
    @keyframes forge-fade-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
    .forge-enter { animation: forge-fade-in 0.4s ease-out forwards; }

    /* Highlight glow */
    .forge-highlight > rect, .forge-highlight > circle, .forge-highlight > path,
    .forge-highlight > line, .forge-highlight > ellipse {
      filter: drop-shadow(0 0 6px var(--forge-hl-color, #E65100));
      stroke: var(--forge-hl-color, #E65100) !important;
      stroke-width: 3 !important;
    }

    /* State badge */
    .forge-state-badge rect { rx: 10; ry: 10; }
    .forge-state-badge text { font-size: 10px; font-weight: 600; text-anchor: middle; }

    /* Frame controls */
    .forge-frame-controls { cursor: pointer; }
    .forge-frame-dot { fill: #ccc; transition: fill 0.2s; }
    .forge-frame-dot--active { fill: #1168BD; }
    .forge-frame-label { font-size: 12px; fill: #555; text-anchor: middle; font-weight: 500; }
"##;

/// Minimal JS for keyboard/click frame navigation (<1KB).
const PLAYBACK_JS: &str = r##"
(function(){
  document.querySelectorAll('.forge-animated').forEach(function(svg){
    var frames=parseInt(svg.dataset.frames||'0'),cur=0;
    function show(n){
      cur=Math.max(0,Math.min(n,frames-1));
      svg.dataset.current=''+cur;
      svg.querySelectorAll('.forge-frame').forEach(function(f){
        f.classList.toggle('forge-frame--active',parseInt(f.dataset.frame)<=cur);
      });
      svg.querySelectorAll('.forge-frame-dot').forEach(function(d,i){
        d.classList.toggle('forge-frame-dot--active',i<=cur);
      });
      var lbl=svg.querySelector('.forge-frame-label');
      if(lbl){
        var active=svg.querySelector('.forge-frame[data-frame="'+cur+'"]');
        lbl.textContent=active?active.dataset.label:'';
      }
    }
    show(0);
    svg.addEventListener('click',function(){show(cur+1>=frames?0:cur+1);});
    document.addEventListener('keydown',function(e){
      if(e.key==='ArrowRight'||e.key===' ')show(cur+1);
      else if(e.key==='ArrowLeft')show(cur-1);
    });
    svg.querySelectorAll('.forge-frame-dot').forEach(function(d,i){
      d.addEventListener('click',function(e){e.stopPropagation();show(i);});
    });
  });
})();
"##;

/// Get the playback JavaScript for embedding in HTML pages.
pub fn playback_script() -> &'static str {
    PLAYBACK_JS
}
