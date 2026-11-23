package dev.horizon.android;

import android.content.ContentUris;
import android.content.Context;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.Build;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.DocumentsProvider;
import android.provider.MediaStore;
import java.io.File;
import java.io.FileNotFoundException;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/** Android SAF adapter only. The allowlist, paths and file operations live in Rust. */
public final class HorizonDocumentsProvider extends DocumentsProvider {
    private static final String RECORDING_MS_PREFIX = "recordings/ms:";
    private static final String RECORDING_FILE_PREFIX = "recordings/file:";
    private static final String[] ROOT_COLUMNS = {
        "root_id", "document_id", "title", "flags", "mime_types", "icon"
    };
    private static final String[] DOCUMENT_COLUMNS = {
        "document_id", "_display_name", "mime_type", "flags", "_size", "last_modified"
    };
    private static native String nativeCall(String home, String operation, String id, String arg)
            throws FileNotFoundException;
    private static native int nativeOpen(String home, String id, String mode)
            throws FileNotFoundException;

    @Override public boolean onCreate() {
        System.loadLibrary("horizon_android");
        return true;
    }
    private String call(String operation, String id, String arg) throws FileNotFoundException {
        return nativeCall(getContext().getFilesDir().getAbsolutePath(), operation, id, arg);
    }

    private static JSONArray recordingRows(Context context) {
        JSONArray rows = new JSONArray();
        if (context == null) {
            return rows;
        }
        String raw = ScreenRecorder.recordings(context);
        if (raw == null || raw.isEmpty()) {
            return rows;
        }
        String[] lines = raw.split("\n");
        for (String line : lines) {
            if (line == null || line.isEmpty()) {
                continue;
            }
            String[] fields = line.split("\t", 4);
            if (fields.length < 4) {
                continue;
            }
            String name = fields[0].trim();
            if (name.isEmpty()) {
                continue;
            }
            long bytes = 0L;
            long added = 0L;
            try { bytes = Long.parseLong(fields[1]); } catch (NumberFormatException ignored) {}
            try { added = Long.parseLong(fields[2]); } catch (NumberFormatException ignored) {}
            String uri = fields[3].trim();
            String docId = null;
            if (uri.startsWith("content://")) {
                int slash = uri.lastIndexOf('/');
                if (slash >= 0) {
                    docId = RECORDING_MS_PREFIX + uri.substring(slash + 1) + ":" + name;
                }
            } else if (uri.startsWith("file:")) {
                docId = RECORDING_FILE_PREFIX + name;
            }
            if (docId == null) {
                continue;
            }
            try {
                JSONObject item = new JSONObject();
                item.put("document_id", docId);
                item.put("_display_name", name);
                item.put("mime_type", "video/mp4");
                item.put("flags", DocumentsContract.Document.FLAG_SUPPORTS_DELETE);
                item.put("_size", bytes);
                item.put("last_modified", added);
                rows.put(item);
            } catch (JSONException ignored) {
            }
        }
        return rows;
    }

    private static JSONObject findVirtualRecording(Context context, String id) {
        if (id == null || (!id.startsWith(RECORDING_MS_PREFIX) && !id.startsWith(RECORDING_FILE_PREFIX))) {
            return null;
        }
        JSONArray rows = recordingRows(context);
        for (int i = 0; i < rows.length(); i++) {
            JSONObject obj = rows.optJSONObject(i);
            if (obj != null && id.equals(obj.optString("document_id"))) {
                return obj;
            }
        }
        return null;
    }

    private static Uri mediaStoreUriFor(String id) {
        if (id == null || !id.startsWith(RECORDING_MS_PREFIX)) {
            return null;
        }
        String rest = id.substring(RECORDING_MS_PREFIX.length());
        int colon = rest.indexOf(':');
        String numStr = colon >= 0 ? rest.substring(0, colon) : rest;
        try {
            long mediaId = Long.parseLong(numStr);
            Uri collection = Build.VERSION.SDK_INT >= 29
                    ? MediaStore.Video.Media.getContentUri(MediaStore.VOLUME_EXTERNAL)
                    : MediaStore.Video.Media.EXTERNAL_CONTENT_URI;
            return ContentUris.withAppendedId(collection, mediaId);
        } catch (NumberFormatException ignored) {
            return null;
        }
    }

    private Cursor query(String operation, String id, String[] projection, String[] defaults)
            throws FileNotFoundException {
        String[] columns = projection == null ? defaults : projection;
        MatrixCursor cursor = new MatrixCursor(columns);
        try {
            JSONArray rows;
            if ("document".equals(operation) && id != null
                    && (id.startsWith(RECORDING_MS_PREFIX) || id.startsWith(RECORDING_FILE_PREFIX))) {
                JSONObject item = findVirtualRecording(getContext(), id);
                if (item == null) {
                    cursor.close();
                    throw new FileNotFoundException("Recording not found");
                }
                rows = new JSONArray().put(item);
            } else {
                rows = new JSONArray(call(operation, id, ""));
                if ("children".equals(operation) && "recordings".equals(id)) {
                    JSONArray extra = recordingRows(getContext());
                    for (int i = 0; i < extra.length(); i++) {
                        JSONObject item = extra.optJSONObject(i);
                        if (item != null) {
                            rows.put(item);
                        }
                    }
                }
            }
            for (int i = 0; i < rows.length(); i++) {
                JSONObject data = rows.getJSONObject(i);
                MatrixCursor.RowBuilder row = cursor.newRow();
                for (String column : columns) {
                    Object value = data.opt(column);
                    if ("icon".equals(column)) value = getContext().getApplicationInfo().icon;
                    row.add(value == JSONObject.NULL ? null : value);
                }
            }
            return cursor;
        } catch (JSONException error) {
            cursor.close();
            throw new FileNotFoundException("Invalid Horizon document response");
        }
    }
    private String changed(String operation, String id, String arg) throws FileNotFoundException {
        if ("delete".equals(operation) && id != null) {
            if (id.startsWith(RECORDING_MS_PREFIX)) {
                Uri uri = mediaStoreUriFor(id);
                if (uri != null) {
                    try {
                        getContext().getContentResolver().delete(uri, null, null);
                    } catch (Throwable ignored) {
                    }
                }
                getContext().getContentResolver().notifyChange(
                        DocumentsContract.buildChildDocumentsUri(
                                getContext().getPackageName() + ".documents", "recordings"), null);
                return id;
            }
            if (id.startsWith(RECORDING_FILE_PREFIX)) {
                String name = id.substring(RECORDING_FILE_PREFIX.length());
                if (!name.isEmpty() && !name.contains("/") && !name.contains("..")) {
                    File folder = RecorderService.legacyFolder(getContext());
                    File file = new File(folder, name);
                    if (file.isFile()) {
                        file.delete();
                    }
                }
                getContext().getContentResolver().notifyChange(
                        DocumentsContract.buildChildDocumentsUri(
                                getContext().getPackageName() + ".documents", "recordings"), null);
                return id;
            }
        }
        String response = call(operation, id, arg);
        getContext().getContentResolver().notifyChange(
                DocumentsContract.buildRootsUri(getContext().getPackageName() + ".documents"), null);
        String parent = id.lastIndexOf('/') < 0 ? "horizon" : id.substring(0, id.lastIndexOf('/'));
        if ("create".equals(operation) || "mkdir".equals(operation)) parent = id;
        getContext().getContentResolver().notifyChange(DocumentsContract.buildChildDocumentsUri(
                getContext().getPackageName() + ".documents", parent), null);
        try { return new JSONArray("[" + response + "]").optString(0, null); }
        catch (JSONException error) { throw new FileNotFoundException("Invalid document ID"); }
    }
    @Override public Cursor queryRoots(String[] projection) throws FileNotFoundException {
        return query("roots", "", projection, ROOT_COLUMNS);
    }
    @Override public Cursor queryDocument(String id, String[] projection) throws FileNotFoundException {
        return query("document", id, projection, DOCUMENT_COLUMNS);
    }
    @Override public Cursor queryChildDocuments(String id, String[] projection, String sortOrder)
            throws FileNotFoundException {
        Cursor cursor = query("children", id, projection, DOCUMENT_COLUMNS);
        Uri children = DocumentsContract.buildChildDocumentsUri(
                getContext().getPackageName() + ".documents", id);
        cursor.setNotificationUri(getContext().getContentResolver(), children);
        return cursor;
    }
    @Override public boolean isChildDocument(String parent, String child) {
        if (("horizon".equals(parent) || "recordings".equals(parent)) && child != null
                && (child.startsWith(RECORDING_MS_PREFIX) || child.startsWith(RECORDING_FILE_PREFIX))) {
            return findVirtualRecording(getContext(), child) != null;
        }
        try { return "true".equals(call("child", parent, child)); }
        catch (FileNotFoundException error) { return false; }
    }
    @Override public DocumentsContract.Path findDocumentPath(String parent, String child)
            throws FileNotFoundException {
        if (child != null && (child.startsWith(RECORDING_MS_PREFIX) || child.startsWith(RECORDING_FILE_PREFIX))) {
            java.util.ArrayList<String> path = new java.util.ArrayList<>();
            if (parent == null || "horizon".equals(parent)) {
                path.add("horizon");
            }
            if (parent == null || "horizon".equals(parent) || "recordings".equals(parent)) {
                path.add("recordings");
            }
            path.add(child);
            return new DocumentsContract.Path(parent == null ? "horizon" : null, path);
        }
        try {
            JSONArray ids = new JSONArray(call("path", child, parent == null ? "" : parent));
            java.util.ArrayList<String> path = new java.util.ArrayList<>();
            for (int i = 0; i < ids.length(); i++) path.add(ids.getString(i));
            return new DocumentsContract.Path(parent == null ? "horizon" : null, path);
        } catch (JSONException error) { throw new FileNotFoundException("Invalid document path"); }
    }
    @Override public ParcelFileDescriptor openDocument(String id, String mode, CancellationSignal signal)
            throws FileNotFoundException {
        if (signal != null) signal.throwIfCanceled();
        if (id != null && id.startsWith(RECORDING_MS_PREFIX)) {
            Uri uri = mediaStoreUriFor(id);
            if (uri == null) {
                throw new FileNotFoundException("Invalid recording URI");
            }
            return getContext().getContentResolver().openFileDescriptor(uri, mode);
        }
        if (id != null && id.startsWith(RECORDING_FILE_PREFIX)) {
            String name = id.substring(RECORDING_FILE_PREFIX.length());
            if (name.isEmpty() || name.contains("/") || name.contains("..")) {
                throw new FileNotFoundException("Invalid recording file");
            }
            File folder = RecorderService.legacyFolder(getContext());
            File file = new File(folder, name);
            if (!file.isFile()) {
                throw new FileNotFoundException("Recording file not found");
            }
            return ParcelFileDescriptor.open(file, ParcelFileDescriptor.parseMode(mode));
        }
        return ParcelFileDescriptor.adoptFd(nativeOpen(
                getContext().getFilesDir().getAbsolutePath(), id, mode));
    }
    @Override public String createDocument(String parent, String mime, String name)
            throws FileNotFoundException {
        return changed(DocumentsContract.Document.MIME_TYPE_DIR.equals(mime) ? "mkdir" : "create",
                parent, name);
    }
    @Override public String renameDocument(String id, String name) throws FileNotFoundException {
        return changed("rename", id, name);
    }
    @Override public void deleteDocument(String id) throws FileNotFoundException {
        changed("delete", id, "");
    }
    @Override public String copyDocument(String sourceDocumentId, String targetParentDocumentId)
            throws FileNotFoundException {
        return changed("copy", sourceDocumentId, targetParentDocumentId);
    }
    @Override public String moveDocument(String sourceDocumentId, String sourceParentDocumentId,
            String targetParentDocumentId) throws FileNotFoundException {
        return changed("move", sourceDocumentId, targetParentDocumentId);
    }
    @Override public void removeDocument(String documentId, String parentDocumentId)
            throws FileNotFoundException {
        deleteDocument(documentId);
    }
}
