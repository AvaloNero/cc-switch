import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  AlertCircle,
  Download,
  GripVertical,
  Loader2,
  Plus,
  RefreshCw,
  RotateCcw,
  Search,
  SquareTerminal,
  Trash2,
  Unplug,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ProviderEmptyState } from "@/components/providers/ProviderEmptyState";
import { ProviderActions } from "@/components/providers/ProviderActions";
import { ProviderIcon } from "@/components/ProviderIcon";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { copilotByokApi } from "@/lib/api";
import type {
  CopilotByokGroup,
  CopilotByokState,
  CopilotByokTargetState,
} from "@/lib/api";
import { cn } from "@/lib/utils";
import { isTextEditableTarget } from "@/utils/domUtils";
import { CopilotByokGroupPanel } from "./CopilotByokGroupPanel";

type BusyAction =
  | "load"
  | "targets"
  | "custom"
  | "group"
  | "sync"
  | "remove"
  | "reorder"
  | "cli"
  | "cli-disable"
  | `import:${string}`
  | `restore:${string}`
  | `custom-remove:${string}`
  | null;

interface CopilotByokSettingsProps {
  mode: "catalog" | "targets";
  onOpenWebsite?: (url: string) => void;
}

export interface CopilotByokSettingsHandle {
  openAdd: () => void;
}

function targetTitle(
  target: CopilotByokTargetState,
  defaultLabel: string,
  appLabel: string,
) {
  if (target.source === "custom") return target.profileName;
  const edition = target.editionName ?? appLabel;
  const profile = target.isDefault ? defaultLabel : target.profileName;
  return `${edition} · ${profile}`;
}

interface SortableCopilotGroupCardProps {
  group: CopilotByokGroup;
  disabled: boolean;
  testing: boolean;
  onEnable: () => void;
  onDisable: () => void;
  onEdit: () => void;
  onDuplicate: () => void;
  onTest: () => void;
  onDelete: () => void;
  onOpenWebsite?: (url: string) => void;
}

function SortableCopilotGroupCard({
  group,
  disabled,
  testing,
  onEnable,
  onDisable,
  onEdit,
  onDuplicate,
  onTest,
  onDelete,
  onOpenWebsite,
}: SortableCopilotGroupCardProps) {
  const { t } = useTranslation();
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: group.id, disabled });
  const description =
    group.notes?.trim() || group.websiteUrl?.trim() || group.url;
  const link =
    group.websiteUrl?.trim() || (!group.notes?.trim() ? group.url : null);

  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={cn(
        "group relative overflow-hidden rounded-xl border border-border bg-card p-4 text-card-foreground transition-all duration-300 hover:border-border-active hover:shadow-sm",
        group.enabled && "border-blue-500/60 shadow-sm shadow-blue-500/10",
        isDragging && "z-10 scale-105 cursor-grabbing border-primary shadow-lg",
      )}
    >
      <div
        className={cn(
          "pointer-events-none absolute inset-0 bg-gradient-to-r from-blue-500/10 to-transparent transition-opacity duration-500",
          group.enabled ? "opacity-100" : "opacity-0",
        )}
      />
      <div className="relative flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <button
            type="button"
            className={cn(
              "-ml-1.5 flex-shrink-0 cursor-grab p-1.5 active:cursor-grabbing",
              "text-muted-foreground/50 transition-colors hover:text-muted-foreground",
              isDragging && "cursor-grabbing",
            )}
            aria-label={t("provider.dragHandle")}
            {...attributes}
            {...listeners}
          >
            <GripVertical className="h-4 w-4" />
          </button>
          <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border bg-muted transition-transform duration-300 group-hover:scale-105">
            <ProviderIcon
              icon={group.icon ?? undefined}
              name={group.name}
              color={group.iconColor ?? undefined}
              size={24}
            />
          </div>
          <div className="min-w-0 flex-1 space-y-1">
            <div className="flex min-h-7 flex-wrap items-center gap-2">
              <h3 className="text-base font-semibold leading-none">
                {group.name}
              </h3>
            </div>
            {link ? (
              <button
                type="button"
                onClick={() => onOpenWebsite?.(link)}
                disabled={!onOpenWebsite}
                className={cn(
                  "inline-flex max-w-full items-center overflow-hidden text-left text-sm text-blue-500 dark:text-blue-400",
                  onOpenWebsite
                    ? "cursor-pointer transition-colors hover:underline"
                    : "cursor-default",
                )}
                title={description}
              >
                <span className="min-w-0 truncate">{description}</span>
              </button>
            ) : (
              <p
                className="truncate text-sm text-muted-foreground"
                title={description}
              >
                {description}
              </p>
            )}
          </div>
        </div>

        <div className="pointer-events-none ml-auto flex flex-shrink-0 items-center gap-1.5 opacity-0 transition-opacity duration-200 group-focus-within:pointer-events-auto group-focus-within:opacity-100 group-hover:pointer-events-auto group-hover:opacity-100">
          <ProviderActions
            appId="copilot-byok"
            isCurrent={false}
            isInConfig={group.enabled}
            isTesting={testing}
            onSwitch={onEnable}
            onRemoveFromConfig={onDisable}
            onEdit={onEdit}
            onDuplicate={onDuplicate}
            onTest={onTest}
            onDelete={onDelete}
          />
        </div>
      </div>
    </div>
  );
}

export const CopilotByokSettings = forwardRef<
  CopilotByokSettingsHandle,
  CopilotByokSettingsProps
>(function CopilotByokSettings({ mode, onOpenWebsite }, ref) {
  const { t } = useTranslation();
  const catalogOnly = mode === "catalog";
  const copy = {
    title: t("apps.copilotByok"),
    description: t("copilotByok.description"),
    refresh: t("common.refresh"),
    loading: t("copilotByok.loading"),
    targets: t("copilotByok.targets"),
    targetsDescription: t("copilotByok.targetsDescription"),
    optInTitle: t("copilotByok.optInTitle"),
    optInDescription: t("copilotByok.optInDescription"),
    noTargets: t("copilotByok.noTargets"),
    defaultProfile: t("copilotByok.defaultProfile"),
    configExists: t("copilotByok.configExists"),
    backupExists: t("copilotByok.backupExists"),
    managedGroups: t("copilotByok.managedGroups"),
    importExisting: t("provider.importCurrent"),
    importWarnings: t("copilotByok.importWarnings"),
    restore: t("common.restore"),
    removeCustom: t("copilotByok.removeCustom"),
    customTarget: t("copilotByok.customTarget"),
    customName: t("copilotByok.customName"),
    customPath: t("copilotByok.customPath"),
    addTarget: t("copilotByok.addTarget"),
    models: t("provider.tabProvider"),
    modelsDescription: t("copilotByok.modelsDescription"),
    addModel: t("provider.addProvider"),
    noModels: t("provider.noProviders"),
    modelCount: t("copilotByok.modelCount"),
    enabled: t("common.enabled"),
    disabled: t("common.disabled"),
    edit: t("common.edit"),
    delete: t("common.delete"),
    copy: t("common.copy"),
    repairSync: t("copilotByok.repairSync"),
    stopManaging: t("copilotByok.stopManaging"),
    selectedProfiles: t("copilotByok.selectedProfiles"),
    enabledModels: t("copilotByok.enabledModels"),
    changedFiles: t("copilotByok.changedFiles"),
    readError: t("copilotByok.readError"),
    saveModelSuccess: t("copilotByok.saveModelSuccess"),
    saveModelLocalSuccess: t("copilotByok.saveModelLocalSuccess"),
    deleteModelSuccess: t("copilotByok.deleteModelSuccess"),
    deleteModelLocalSuccess: t("copilotByok.deleteModelLocalSuccess"),
    targetUpdateSuccess: t("copilotByok.targetUpdateSuccess"),
    syncSuccess: t("copilotByok.syncSuccess"),
    stopSuccess: t("copilotByok.stopSuccess"),
    restoreSuccess: t("copilotByok.restoreSuccess"),
    restoreNoop: t("copilotByok.restoreNoop"),
    customAdded: t("copilotByok.customAdded"),
    customRemoved: t("copilotByok.customRemoved"),
    confirmDelete: t("copilotByok.confirmDelete"),
    confirmStop: t("copilotByok.confirmStop"),
    cliTitle: t("copilotByok.cli.title"),
    cliDescription: t("copilotByok.cli.description"),
    cliProvider: t("copilotByok.cli.provider"),
    cliModel: t("copilotByok.cli.model"),
    cliApply: t("copilotByok.cli.apply"),
    cliDisable: t("copilotByok.cli.disable"),
    cliActive: t("copilotByok.cli.active"),
    cliInactive: t("copilotByok.cli.inactive"),
    cliNeedsApply: t("copilotByok.cli.needsApply"),
    cliUnsupported: t("copilotByok.cli.unsupported"),
    cliRestart: t("copilotByok.cli.restart"),
    cliSecurity: t("copilotByok.cli.security"),
    cliConflict: t("copilotByok.cli.conflict"),
    cliApplySuccess: t("copilotByok.cli.applySuccess"),
    cliDisableSuccess: t("copilotByok.cli.disableSuccess"),
  };
  const [state, setState] = useState<CopilotByokState | null>(null);
  const [busy, setBusy] = useState<BusyAction>("load");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingGroup, setEditingGroup] = useState<CopilotByokGroup | null>(
    null,
  );
  const [customName, setCustomName] = useState("");
  const [customPath, setCustomPath] = useState("");
  const [cliGroupId, setCliGroupId] = useState("");
  const [cliModelId, setCliModelId] = useState("");
  const [testingGroupId, setTestingGroupId] = useState<string | null>(null);
  const [pendingDeleteGroup, setPendingDeleteGroup] =
    useState<CopilotByokGroup | null>(null);
  const [stopConfirmOpen, setStopConfirmOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  const load = useCallback(
    async (showToast = false) => {
      setBusy("load");
      try {
        const next = await copilotByokApi.getState();
        setState(next);
        if (showToast) toast.success(copy.refresh);
      } catch (error) {
        console.error("[CopilotByokSettings] Failed to load", error);
        toast.error(String(error));
      } finally {
        setBusy(null);
      }
    },
    [copy.refresh],
  );

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!state) return;
    const groupId =
      state.cli.selectedGroupId ??
      state.groups.find((group) => group.models.length > 0)?.id ??
      "";
    const group = state.groups.find((item) => item.id === groupId);
    const modelId =
      state.cli.selectedGroupId === groupId
        ? (state.cli.selectedModelId ?? group?.models[0]?.id ?? "")
        : (group?.models[0]?.id ?? "");
    setCliGroupId(groupId);
    setCliModelId(modelId);
  }, [state]);

  useEffect(() => {
    if (!catalogOnly) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const key = event.key.toLowerCase();
      if ((event.metaKey || event.ctrlKey) && key === "f") {
        if (isTextEditableTarget(document.activeElement)) return;
        event.preventDefault();
        setIsSearchOpen(true);
      } else if (key === "escape") {
        setIsSearchOpen(false);
      }
    };
    globalThis.addEventListener("keydown", handleKeyDown);
    return () => globalThis.removeEventListener("keydown", handleKeyDown);
  }, [catalogOnly]);

  useEffect(() => {
    if (!isSearchOpen) return;
    const frame = requestAnimationFrame(() => {
      searchInputRef.current?.focus();
      searchInputRef.current?.select();
    });
    return () => cancelAnimationFrame(frame);
  }, [isSearchOpen]);

  const selectedTargets = useMemo(
    () => state?.targets.filter((target) => target.selected) ?? [],
    [state],
  );

  const selectedCliGroup = useMemo(
    () => state?.groups.find((group) => group.id === cliGroupId) ?? null,
    [cliGroupId, state?.groups],
  );

  const filteredGroups = useMemo(() => {
    const keyword = searchTerm.trim().toLowerCase();
    if (!keyword) return state?.groups ?? [];
    return (state?.groups ?? []).filter((group) =>
      [group.name, group.notes, group.websiteUrl, group.url].some((value) =>
        value?.toLowerCase().includes(keyword),
      ),
    );
  }, [searchTerm, state?.groups]);

  const importTarget = useMemo(() => {
    const targets = state?.targets.filter((target) => !target.readError) ?? [];
    return (
      targets.find((target) => target.selected && target.configExists) ??
      targets.find((target) => target.isDefault && target.configExists) ??
      targets.find((target) => target.configExists) ??
      targets.find((target) => target.selected) ??
      targets.find((target) => target.isDefault) ??
      targets[0] ??
      null
    );
  }, [state]);

  const openAdd = useCallback(() => {
    setEditingGroup(null);
    setEditorOpen(true);
  }, []);

  useImperativeHandle(ref, () => ({ openAdd }), [openAdd]);

  const updateTargets = async (targetId: string, checked: boolean) => {
    if (!state || busy) return;
    setBusy("targets");
    try {
      const nextIds = checked
        ? [...new Set([...state.selectedTargetIds, targetId])]
        : state.selectedTargetIds.filter((id) => id !== targetId);
      const next = await copilotByokApi.setTargets(nextIds);
      setState(next);
      toast.success(copy.targetUpdateSuccess);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const addCustomTarget = async () => {
    if (!customPath.trim() || busy) return;
    setBusy("custom");
    try {
      const next = await copilotByokApi.addCustomTarget(
        customPath.trim(),
        customName.trim() || null,
      );
      setState(next);
      setCustomName("");
      setCustomPath("");
      toast.success(copy.customAdded);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const removeCustomTarget = async (target: CopilotByokTargetState) => {
    if (busy) return;
    setBusy(`custom-remove:${target.id}`);
    try {
      const next = await copilotByokApi.removeCustomTarget(target.id);
      setState(next);
      toast.success(copy.customRemoved);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const saveGroup = async (group: CopilotByokGroup) => {
    const shouldSync = selectedTargets.length > 0;
    setBusy("group");
    try {
      const next = await copilotByokApi.upsertGroup(group);
      setState(next);
      setEditorOpen(false);
      toast.success(
        shouldSync ? copy.saveModelSuccess : copy.saveModelLocalSuccess,
      );
    } catch (error) {
      toast.error(String(error));
      throw error;
    } finally {
      setBusy(null);
    }
  };

  const toggleGroup = async (group: CopilotByokGroup, enabled: boolean) => {
    if (busy) return;
    setBusy("group");
    try {
      const next = await copilotByokApi.upsertGroup({ ...group, enabled });
      setState(next);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const deleteGroup = async (group: CopilotByokGroup) => {
    if (busy) return;
    setPendingDeleteGroup(group);
  };

  const confirmDeleteGroup = async () => {
    const group = pendingDeleteGroup;
    if (!group || busy) return;
    const shouldSync = selectedTargets.length > 0;
    setBusy("group");
    try {
      const next = await copilotByokApi.deleteGroup(group.id);
      setState(next);
      setPendingDeleteGroup(null);
      toast.success(
        shouldSync ? copy.deleteModelSuccess : copy.deleteModelLocalSuccess,
      );
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const duplicateGroup = async (group: CopilotByokGroup) => {
    if (!state || busy) return;
    setBusy("group");
    try {
      const existingNames = new Set(
        state.groups.map((item) => item.name.toLowerCase()),
      );
      const baseName = `${group.name} ${copy.copy}`;
      let name = baseName;
      let suffix = 2;
      while (existingNames.has(name.toLowerCase())) {
        name = `${baseName} ${suffix}`;
        suffix += 1;
      }
      const duplicateId = crypto.randomUUID();
      const duplicate: CopilotByokGroup = {
        ...structuredClone(group),
        id: duplicateId,
        name,
        enabled: false,
        models: group.models.map((model) => ({
          ...structuredClone(model),
          id: crypto.randomUUID(),
        })),
      };
      const added = await copilotByokApi.upsertGroup(duplicate);
      setState(added);
      toast.success(
        selectedTargets.length > 0
          ? copy.saveModelSuccess
          : copy.saveModelLocalSuccess,
      );
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const testGroup = async (group: CopilotByokGroup) => {
    if (testingGroupId) return;
    setTestingGroupId(group.id);
    try {
      const result = await copilotByokApi.checkConnection(group.id);
      if (result.status === "operational") {
        toast.success(
          t("streamCheck.reachable", {
            providerName: group.name,
            responseTimeMs: result.responseTimeMs,
          }),
        );
      } else if (result.status === "degraded") {
        toast.warning(
          t("streamCheck.reachableSlow", {
            providerName: group.name,
            responseTimeMs: result.responseTimeMs,
          }),
        );
      } else {
        toast.error(
          t("streamCheck.unreachable", {
            providerName: group.name,
            message: result.message,
          }),
        );
      }
    } catch (error) {
      toast.error(
        t("streamCheck.error", {
          providerName: group.name,
          error: String(error),
        }),
      );
    } finally {
      setTestingGroupId(null);
    }
  };

  const handleGroupDragEnd = async ({ active, over }: DragEndEvent) => {
    if (!state || busy || !over || active.id === over.id) return;
    const oldIndex = state.groups.findIndex(
      (group) => group.id === String(active.id),
    );
    const newIndex = state.groups.findIndex(
      (group) => group.id === String(over.id),
    );
    if (oldIndex < 0 || newIndex < 0) return;

    const previousGroups = state.groups;
    const reordered = arrayMove(state.groups, oldIndex, newIndex);
    setState({ ...state, groups: reordered });
    setBusy("reorder");
    try {
      const next = await copilotByokApi.reorderGroups(
        reordered.map((group) => group.id),
      );
      setState(next);
      toast.success(t("provider.sortUpdated"));
    } catch (error) {
      setState((current) =>
        current ? { ...current, groups: previousGroups } : current,
      );
      toast.error(t("provider.sortUpdateFailed"), {
        description: String(error),
      });
    } finally {
      setBusy(null);
    }
  };

  const importExistingModels = async (target: CopilotByokTargetState) => {
    if (busy) return;
    setBusy(`import:${target.id}`);
    try {
      const result = await copilotByokApi.importModels(target.id);
      if (result.importedModelCount > 0 || result.reusedModelCount > 0) {
        toast.success(t("provider.importCurrentDescription"));
      } else {
        toast.info(t("provider.noProviders"));
      }
      if (result.warnings.length > 0) {
        toast.warning(copy.importWarnings, {
          description: result.warnings.join("\n"),
        });
      }
      await load();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const sync = async () => {
    if (busy || selectedTargets.length === 0) return;
    setBusy("sync");
    try {
      const result = await copilotByokApi.sync();
      toast.success(
        `${copy.syncSuccess} · ${copy.changedFiles}: ${result.changedTargetCount}`,
      );
      await load();
    } catch (error) {
      toast.error(String(error));
      setBusy(null);
    }
  };

  const stopManaging = async () => {
    if (!state || busy || selectedTargets.length === 0) return;
    setStopConfirmOpen(true);
  };

  const confirmStopManaging = async () => {
    if (!state || busy || selectedTargets.length === 0) return;
    setBusy("remove");
    try {
      const next = await copilotByokApi.setTargets([]);
      setState(next);
      setStopConfirmOpen(false);
      toast.success(copy.stopSuccess);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const restoreTarget = async (target: CopilotByokTargetState) => {
    if (busy) return;
    setBusy(`restore:${target.id}`);
    try {
      const restored = await copilotByokApi.restoreBackup(target.id);
      toast.success(restored ? copy.restoreSuccess : copy.restoreNoop);
      await load();
    } catch (error) {
      toast.error(String(error));
      setBusy(null);
    }
  };

  const applyCliSelection = async () => {
    if (!state?.cli.supported || !cliGroupId || !cliModelId || busy) return;
    setBusy("cli");
    try {
      const next = await copilotByokApi.setCliSelection(cliGroupId, cliModelId);
      setState(next);
      toast.success(copy.cliApplySuccess);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const disableCli = async () => {
    if (!state?.cli.supported || !state.cli.enabled || busy) return;
    setBusy("cli-disable");
    try {
      const next = await copilotByokApi.disableCli();
      setState(next);
      toast.success(copy.cliDisableSuccess);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  if (!state && busy === "load") {
    return (
      <div className="flex items-center justify-center gap-2 py-12 text-sm text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        {copy.loading}
      </div>
    );
  }

  if (!state) {
    return (
      <Alert variant="destructive">
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>{copy.readError}</AlertTitle>
        <AlertDescription>
          <Button className="mt-3" variant="outline" onClick={() => load()}>
            {copy.refresh}
          </Button>
        </AlertDescription>
      </Alert>
    );
  }

  const groupEditor = (
    <CopilotByokGroupPanel
      open={editorOpen}
      group={editingGroup}
      saving={busy === "group"}
      onOpenChange={(open) => {
        setEditorOpen(open);
        if (!open) setEditingGroup(null);
      }}
      onSave={saveGroup}
    />
  );

  const confirmationDialogs = (
    <>
      <ConfirmDialog
        isOpen={pendingDeleteGroup !== null}
        title={t("confirm.deleteProvider")}
        message={copy.confirmDelete}
        pending={busy === "group"}
        onConfirm={() => void confirmDeleteGroup()}
        onCancel={() => setPendingDeleteGroup(null)}
      />
      <ConfirmDialog
        isOpen={stopConfirmOpen}
        title={copy.stopManaging}
        message={copy.confirmStop}
        pending={busy === "remove"}
        onConfirm={() => void confirmStopManaging()}
        onCancel={() => setStopConfirmOpen(false)}
      />
    </>
  );

  if (catalogOnly) {
    return (
      <>
        <div className="mt-4 space-y-3">
          <AnimatePresence>
            {isSearchOpen && (
              <motion.div
                key="copilot-provider-search"
                initial={{ opacity: 0, y: -8, scale: 0.98 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, y: -8, scale: 0.98 }}
                transition={{ duration: 0.18, ease: "easeOut" }}
                className="fixed left-1/2 top-[6.5rem] z-40 w-[min(90vw,26rem)] -translate-x-1/2 sm:left-auto sm:right-6 sm:translate-x-0"
              >
                <div className="space-y-3 rounded-2xl border border-white/10 bg-background/95 p-4 shadow-md shadow-black/20 backdrop-blur-md">
                  <div className="relative flex items-center gap-2">
                    <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      ref={searchInputRef}
                      value={searchTerm}
                      onChange={(event) => setSearchTerm(event.target.value)}
                      placeholder={t("provider.searchPlaceholder")}
                      aria-label={t("provider.searchAriaLabel")}
                      className="pl-9 pr-16"
                    />
                    {searchTerm && (
                      <Button
                        variant="ghost"
                        size="sm"
                        className="absolute right-11 top-1/2 -translate-y-1/2 text-xs"
                        onClick={() => setSearchTerm("")}
                      >
                        {t("common.clear")}
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="icon"
                      className="ml-auto"
                      onClick={() => setIsSearchOpen(false)}
                      aria-label={t("provider.searchCloseAriaLabel")}
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                  <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-muted-foreground">
                    <span>{t("provider.searchScopeHint")}</span>
                    <span>{t("provider.searchCloseHint")}</span>
                  </div>
                </div>
              </motion.div>
            )}
          </AnimatePresence>

          {state.groups.length === 0 ? (
            <ProviderEmptyState
              appId="copilot-byok"
              onCreate={openAdd}
              onImport={
                importTarget
                  ? () => void importExistingModels(importTarget)
                  : undefined
              }
            />
          ) : filteredGroups.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border px-6 py-8 text-center text-sm text-muted-foreground">
              {t("provider.noSearchResults")}
            </div>
          ) : (
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={(event) => void handleGroupDragEnd(event)}
            >
              <SortableContext
                items={filteredGroups.map((group) => group.id)}
                strategy={verticalListSortingStrategy}
              >
                <div className="space-y-3">
                  {filteredGroups.map((group) => (
                    <SortableCopilotGroupCard
                      key={group.id}
                      group={group}
                      disabled={Boolean(busy)}
                      testing={testingGroupId === group.id}
                      onEnable={() => void toggleGroup(group, true)}
                      onDisable={() => void toggleGroup(group, false)}
                      onEdit={() => {
                        if (busy) return;
                        setEditingGroup(group);
                        setEditorOpen(true);
                      }}
                      onDuplicate={() => void duplicateGroup(group)}
                      onTest={() => void testGroup(group)}
                      onDelete={() => void deleteGroup(group)}
                      onOpenWebsite={onOpenWebsite}
                    />
                  ))}
                </div>
              </SortableContext>
            </DndContext>
          )}
        </div>
        {groupEditor}
        {confirmationDialogs}
      </>
    );
  }

  const cliDraftMatches =
    state.cli.enabled &&
    state.cli.selectedGroupId === cliGroupId &&
    state.cli.selectedModelId === cliModelId &&
    state.cli.environmentMatches;

  return (
    <div className="space-y-3">
      {selectedTargets.length === 0 ? (
        <div className="flex gap-3 rounded-xl border border-border bg-card p-4">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0">
            <p className="text-[13px] font-medium leading-5">
              {copy.optInTitle}
            </p>
            <p className="mt-1 text-[13px] leading-5 text-muted-foreground">
              {copy.optInDescription}
            </p>
          </div>
        </div>
      ) : null}

      <section
        aria-labelledby="copilot-byok-cli"
        className="overflow-hidden rounded-xl border border-border bg-card"
      >
        <div className="flex flex-col gap-3 border-b border-border/60 px-4 py-4 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <SquareTerminal className="h-4 w-4 text-muted-foreground" />
              <h3
                id="copilot-byok-cli"
                className="text-base font-semibold leading-none"
              >
                {copy.cliTitle}
              </h3>
              <Badge
                variant={cliDraftMatches ? "secondary" : "outline"}
                className="h-5 px-1.5 text-[10px] font-medium"
              >
                {!state.cli.supported
                  ? copy.cliUnsupported
                  : cliDraftMatches
                    ? copy.cliActive
                    : state.cli.enabled
                      ? copy.cliNeedsApply
                      : copy.cliInactive}
              </Badge>
            </div>
            <p className="mt-1.5 text-xs leading-5 text-muted-foreground">
              {copy.cliDescription}
            </p>
          </div>
        </div>

        <div className="space-y-4 px-5 py-5">
          <div className="grid gap-3 md:grid-cols-2">
            <div className="space-y-2">
              <Label className="text-xs font-medium text-muted-foreground">
                {copy.cliProvider}
              </Label>
              <Select
                value={cliGroupId}
                disabled={Boolean(busy) || !state.cli.supported}
                onValueChange={(value) => {
                  const group = state.groups.find((item) => item.id === value);
                  setCliGroupId(value);
                  setCliModelId(group?.models[0]?.id ?? "");
                }}
              >
                <SelectTrigger>
                  <SelectValue placeholder={copy.cliProvider} />
                </SelectTrigger>
                <SelectContent>
                  {state.groups.map((group) => (
                    <SelectItem
                      key={group.id}
                      value={group.id}
                      disabled={group.models.length === 0}
                    >
                      {group.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label className="text-xs font-medium text-muted-foreground">
                {copy.cliModel}
              </Label>
              <Select
                value={cliModelId}
                disabled={
                  Boolean(busy) || !state.cli.supported || !selectedCliGroup
                }
                onValueChange={setCliModelId}
              >
                <SelectTrigger>
                  <SelectValue placeholder={copy.cliModel} />
                </SelectTrigger>
                <SelectContent>
                  {selectedCliGroup?.models.map((model) => (
                    <SelectItem key={model.id} value={model.id}>
                      {model.name} · {model.modelId}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {state.cli.environmentConflicts.length > 0 ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>{copy.cliConflict}</AlertTitle>
              <AlertDescription className="break-all font-mono text-xs">
                {state.cli.environmentConflicts.join(", ")}
              </AlertDescription>
            </Alert>
          ) : null}

          <div className="space-y-1 text-[11px] leading-4 text-muted-foreground">
            <p>{copy.cliRestart}</p>
            <p>{copy.cliSecurity}</p>
          </div>

          <div className="flex flex-wrap justify-end gap-2">
            {state.cli.enabled ? (
              <Button
                size="sm"
                variant="ghost"
                className="h-8 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive"
                disabled={Boolean(busy) || !state.cli.supported}
                onClick={() => void disableCli()}
              >
                {busy === "cli-disable" ? (
                  <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                ) : (
                  <RotateCcw className="mr-2 h-3.5 w-3.5" />
                )}
                {copy.cliDisable}
              </Button>
            ) : null}
            <Button
              size="sm"
              className="h-8 text-xs"
              disabled={
                Boolean(busy) ||
                !state.cli.supported ||
                !cliGroupId ||
                !cliModelId ||
                cliDraftMatches
              }
              onClick={() => void applyCliSelection()}
            >
              {busy === "cli" ? (
                <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
              ) : (
                <SquareTerminal className="mr-2 h-3.5 w-3.5" />
              )}
              {copy.cliApply}
            </Button>
          </div>
        </div>
      </section>

      <section
        aria-labelledby="copilot-byok-targets"
        className="overflow-hidden rounded-xl border border-border bg-card"
      >
        <div className="border-b border-border/60 px-4 py-4">
          <h3
            id="copilot-byok-targets"
            className="text-base font-semibold leading-none"
          >
            {copy.targets}
          </h3>
          <p className="mt-1.5 text-xs leading-5 text-muted-foreground">
            {copy.targetsDescription}
          </p>
        </div>

        <div className="divide-y divide-border/60">
          {state.targets.length === 0 ? (
            <p className="px-5 py-6 text-[13px] leading-5 text-muted-foreground">
              {copy.noTargets}
            </p>
          ) : (
            <>
              {state.targets.map((target) => (
                <div
                  key={target.id}
                  className="flex flex-col gap-4 px-5 py-5 md:flex-row md:items-center md:justify-between"
                >
                  <div className="flex min-w-0 items-start gap-3">
                    <Checkbox
                      checked={target.selected}
                      disabled={Boolean(busy)}
                      onCheckedChange={(checked) =>
                        void updateTargets(target.id, checked === true)
                      }
                      aria-label={targetTitle(
                        target,
                        copy.defaultProfile,
                        copy.title,
                      )}
                    />
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-1.5">
                        <span className="text-[13px] font-medium leading-5">
                          {targetTitle(target, copy.defaultProfile, copy.title)}
                        </span>
                        {target.configExists ? (
                          <Badge
                            variant="secondary"
                            className="h-5 px-1.5 text-[10px] font-medium"
                          >
                            {copy.configExists}
                          </Badge>
                        ) : null}
                        {target.backupExists ? (
                          <Badge
                            variant="outline"
                            className="h-5 px-1.5 text-[10px] font-medium"
                          >
                            {copy.backupExists}
                          </Badge>
                        ) : null}
                        {target.managedGroupCount > 0 ? (
                          <Badge
                            variant="outline"
                            className="h-5 px-1.5 text-[10px] font-medium"
                          >
                            {copy.managedGroups}: {target.managedGroupCount}
                          </Badge>
                        ) : null}
                      </div>
                      <p className="mt-1.5 break-all font-mono text-[11px] leading-4 text-muted-foreground">
                        {target.languageModelsPath}
                      </p>
                      {target.readError ? (
                        <p className="mt-2 text-[11px] leading-4 text-destructive">
                          {copy.readError}: {target.readError}
                        </p>
                      ) : null}
                    </div>
                  </div>

                  <div className="flex shrink-0 flex-wrap items-center gap-2">
                    {target.configExists && !target.readError ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        disabled={Boolean(busy)}
                        onClick={() => void importExistingModels(target)}
                        className="h-8 text-xs"
                      >
                        {busy === `import:${target.id}` ? (
                          <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                        ) : (
                          <Download className="mr-2 h-3.5 w-3.5" />
                        )}
                        {copy.importExisting}
                      </Button>
                    ) : null}
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={Boolean(busy)}
                      onClick={() => void restoreTarget(target)}
                      className="h-8 text-xs text-muted-foreground"
                    >
                      {busy === `restore:${target.id}` ? (
                        <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                      ) : (
                        <RotateCcw className="mr-2 h-3.5 w-3.5" />
                      )}
                      {copy.restore}
                    </Button>
                    {target.source === "custom" ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={Boolean(busy)}
                        onClick={() => void removeCustomTarget(target)}
                        className="h-8 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive"
                      >
                        {busy === `custom-remove:${target.id}` ? (
                          <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                        ) : (
                          <Trash2 className="mr-2 h-3.5 w-3.5" />
                        )}
                        {copy.removeCustom}
                      </Button>
                    ) : null}
                  </div>
                </div>
              ))}
            </>
          )}

          <div className="px-5 py-5">
            <p className="text-[13px] font-medium leading-5">
              {copy.customTarget}
            </p>
            <div className="mt-4 grid gap-3 md:grid-cols-[minmax(0,0.8fr)_minmax(0,1.5fr)]">
              <div className="space-y-2">
                <Label
                  htmlFor="copilot-custom-name"
                  className="text-xs font-medium text-muted-foreground"
                >
                  {copy.customName}
                </Label>
                <Input
                  id="copilot-custom-name"
                  value={customName}
                  onChange={(event) => setCustomName(event.target.value)}
                  disabled={Boolean(busy)}
                  className="h-9 bg-background/60 text-[13px]"
                />
              </div>
              <div className="space-y-2">
                <Label
                  htmlFor="copilot-custom-path"
                  className="text-xs font-medium text-muted-foreground"
                >
                  {copy.customPath}
                </Label>
                <Input
                  id="copilot-custom-path"
                  value={customPath}
                  onChange={(event) => setCustomPath(event.target.value)}
                  disabled={Boolean(busy)}
                  className="h-9 bg-background/60 font-mono text-xs"
                />
              </div>
              <div className="flex justify-end md:col-span-2">
                <Button
                  size="sm"
                  onClick={() => void addCustomTarget()}
                  disabled={Boolean(busy) || customPath.trim().length === 0}
                  className="h-8 text-xs"
                >
                  {busy === "custom" ? (
                    <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <Plus className="mr-1.5 h-3.5 w-3.5" />
                  )}
                  {copy.addTarget}
                </Button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <div className="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-border bg-card p-4">
        <div className="flex flex-wrap gap-4 text-[12px] text-muted-foreground">
          <span>
            {copy.selectedProfiles}: {selectedTargets.length}
          </span>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => void sync()}
            disabled={Boolean(busy) || selectedTargets.length === 0}
            className="h-8 text-xs text-muted-foreground"
          >
            {busy === "sync" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 h-4 w-4" />
            )}
            {copy.repairSync}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => void stopManaging()}
            disabled={Boolean(busy) || selectedTargets.length === 0}
            className="h-8 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive"
          >
            {busy === "remove" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Unplug className="mr-2 h-4 w-4" />
            )}
            {copy.stopManaging}
          </Button>
        </div>
      </div>

      {confirmationDialogs}
    </div>
  );
});
