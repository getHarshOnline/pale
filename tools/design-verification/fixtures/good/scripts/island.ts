import { animate, stagger } from "motion";
const THRESHOLD = 0.35; // behavioural constant: fraction of the region in view before it runs
const io = new IntersectionObserver(start, { threshold: THRESHOLD, rootMargin: "0px 0px -10% 0px" });
const d = seconds(read(el, "duration-expressive"));
animate(node, { x: 0 }, { duration: d, delay: stagger(step), ease: easing });
