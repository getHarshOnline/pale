import { animate, stagger } from "motion";
animate(edges, { opacity: 1 }, { duration: 0.35, delay: stagger(0.04) });
animate(node, { x: 0 }, { type: "spring", visualDuration: 0.4, bounce: 0.2, ease: [0.3, 0.9, 0.1, 1] });
el.style.transition = "opacity 200ms ease";
