(() => {
  const statusEl = document.getElementById("copy-status");
  const palette = document.getElementById("commandPalette");

  const track = (name) => {
    if (typeof window.va === "function") {
      window.va("event", { name });
    }
  };

  const setStatus = (message) => {
    if (statusEl) {
      statusEl.textContent = message;
    }
  };

  const copyText = async (text) => {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }

    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "absolute";
    area.style.left = "-9999px";
    document.body.appendChild(area);
    area.select();
    document.execCommand("copy");
    document.body.removeChild(area);
  };

  document.querySelectorAll("[data-copy-target]").forEach((button) => {
    button.addEventListener("click", async () => {
      const targetId = button.getAttribute("data-copy-target");
      const source = targetId ? document.getElementById(targetId) : null;
      const text = source?.textContent?.trim() ?? "";

      if (!text) {
        setStatus("Install command unavailable.");
        return;
      }

      try {
        await copyText(text);
        setStatus("Command copied.");
      } catch {
        setStatus("Copy failed. Select manually.");
      }
    });
  });

  const openPalette = () => {
    if (palette && typeof palette.showModal === "function") {
      palette.showModal();
    }
  };

  if (palette) {
    document.querySelectorAll("[data-open-command]").forEach((node) => {
      node.addEventListener("click", openPalette);
    });
  }

  document.addEventListener("keydown", (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      openPalette();
      return;
    }

    if (event.key === "Escape" && palette?.open) {
      palette.close();
    }
  });

  document.querySelectorAll("[data-track-primary]").forEach((cta) => {
    cta.addEventListener("click", () => {
      track("lp_click_install_primary");
    });
  });

  track("lp_view");
})();
