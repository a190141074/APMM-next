// 密码通过非受控输入读取一次，提交后立即清空，不放入 React 状态和任务参数。
import { useRef } from "react";
import { Modal } from "../../shared/Ui";
import { useActions } from "../../hooks/useApp";
import { authorService } from "../../core/services/author";
export function CredentialDialog({
  workId,
  onClose,
}: {
  workId: string | null;
  onClose: () => void;
}) {
  const password = useRef<HTMLInputElement>(null);
  const { api, act, busy, notice } = useActions();
  return (
    <Modal
      open
      title={workId ? "备份作品管理凭证" : "恢复作品管理凭证"}
      onClose={onClose}
    >
      <div className="wizard-body">
        <p>每件作品独立管理。请妥善保存加密备份和密码。</p>
        <label className="field">
          备份密码
          <input
            ref={password}
            type="password"
            autoComplete="off"
            minLength={10}
          />
        </label>
        {notice && <p role="status">{notice}</p>}
        <button
          className="button primary"
          disabled={busy}
          onClick={() =>
            void act(async () => {
              const secret = password.current?.value || "";
              if (password.current) password.current.value = "";
              if (workId) {
                const destination = await api.saveFile(
                  `workshop-${workId}.apmmkey`,
                );
                if (destination)
                  await authorService.backup(workId, destination, secret);
                else return;
              } else {
                const files = await api.chooseFiles("credentials");
                const path = Array.isArray(files) ? files[0] : files;
                if (path) await authorService.restore(path, secret);
                else return;
              }
              onClose();
            })
          }
        >
          {workId ? "选择位置并备份" : "选择备份并恢复"}
        </button>
      </div>
    </Modal>
  );
}
