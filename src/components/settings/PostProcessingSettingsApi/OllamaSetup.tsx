import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands, type OllamaStatus } from "@/bindings";
import { Button } from "../../ui/Button";
import { Dropdown } from "../../ui/Dropdown";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";

const CTX_OPTIONS = [2048, 4096, 8192, 16384];

const Chip: React.FC<{ ok: boolean; label: string }> = ({ ok, label }) => (
  <span
    className={`rounded-full px-2 py-0.5 text-xs ${ok ? "bg-logo-primary text-white" : "bg-mid-gray/30 text-text"}`}
  >
    {label}
  </span>
);

export const OllamaSetup: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const [status, setStatus] = useState<OllamaStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<string>("");

  const refresh = async () => {
    const s = await commands.ollamaStatus();
    setStatus(s);
  };

  useEffect(() => {
    void refresh();

    const unP = listen<{ status: string; percent: number | null }>(
      "ollama-pull-progress",
      (e) =>
        setProgress(
          `${e.payload.status}${e.payload.percent != null ? ` ${e.payload.percent}%` : ""}`,
        ),
    );
    const unL = listen<string>("ollama-log", (e) => setProgress(e.payload));

    return () => {
      void unP.then((f) => f());
      void unL.then((f) => f());
    };
  }, []);

  const setup = async () => {
    setBusy(true);
    try {
      await commands.ollamaEnsureReady();
      await refresh();
    } finally {
      setBusy(false);
      setProgress("");
    }
  };

  const model = getSetting("ollama_model") ?? "llama3.2:3b";
  const numCtx = getSetting("ollama_num_ctx") ?? 4096;

  return (
    <SettingContainer
      title={t("settings.postProcessing.api.ollama.title")}
      description={""}
      descriptionMode="inline"
      layout="stacked"
      grouped={true}
    >
      <div className="flex flex-col gap-3">
        <div className="flex gap-2">
          <Chip
            ok={!!status?.installed}
            label={t("settings.postProcessing.api.ollama.installed")}
          />
          <Chip
            ok={!!status?.running}
            label={t("settings.postProcessing.api.ollama.running")}
          />
          <Chip
            ok={!!status?.has_model}
            label={t("settings.postProcessing.api.ollama.modelReady")}
          />
        </div>

        <div className="flex items-center gap-2">
          <Button onClick={() => void setup()} disabled={busy}>
            {busy
              ? t("settings.postProcessing.api.ollama.working")
              : t("settings.postProcessing.api.ollama.setup")}
          </Button>
          {progress && (
            <span className="text-xs text-mid-gray">{progress}</span>
          )}
        </div>

        <div className="flex items-center gap-2">
          <label className="text-sm">
            {t("settings.postProcessing.api.ollama.model")}
          </label>
          <Dropdown
            selectedValue={model ?? null}
            onSelect={(v) => void updateSetting("ollama_model", v)}
            options={(status?.models?.length
              ? status.models
              : [model ?? "llama3.2:3b"]
            ).map((m) => ({ value: m, label: m }))}
          />
        </div>

        <div className="flex items-center gap-2">
          <label className="text-sm">
            {t("settings.postProcessing.api.ollama.contextLength")}
          </label>
          <Dropdown
            selectedValue={String(numCtx ?? 4096)}
            onSelect={(v) => void updateSetting("ollama_num_ctx", Number(v))}
            options={CTX_OPTIONS.map((n) => ({
              value: String(n),
              label: String(n),
            }))}
          />
        </div>
      </div>
    </SettingContainer>
  );
};
