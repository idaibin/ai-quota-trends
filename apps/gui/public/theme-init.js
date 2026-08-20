(function () {
  try {
    var params = new URLSearchParams(window.location.search);
    var surface = params.get("surface") || "main";
    document.documentElement.dataset.surface = surface;
    var cached = localStorage.getItem("aqt:cached-settings");
    var theme = "light";
    if (cached) {
      try {
        var parsed = JSON.parse(cached);
        if (parsed && (parsed.theme === "dark" || parsed.theme === "light")) {
          theme = parsed.theme;
        } else if (parsed && parsed.theme === "system") {
          theme =
            window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches
              ? "dark"
              : "light";
        } else if (window.matchMedia) {
          theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
        }
      } catch (e) {
        if (window.matchMedia) {
          theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
        }
      }
    } else if (window.matchMedia) {
      theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    document.documentElement.dataset.theme = theme;
  } catch (e) {}
})();
