import { greet } from "./util.js";
import "./style.css";

document.querySelector("#app").textContent = greet("bundle");
