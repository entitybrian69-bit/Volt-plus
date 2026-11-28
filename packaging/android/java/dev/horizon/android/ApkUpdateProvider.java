package dev.horizon.android;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.nio.file.Files;
import java.util.Locale;

/**
 * Private content provider that exposes downloaded release APKs strictly inside
 * {@code getCacheDir()/updates} to Android's system Package Installer.
 */
public final class ApkUpdateProvider extends ContentProvider {
    private static final String APK_MIME = "application/vnd.android.package-archive";
    private static final String[] DEFAULT_COLUMNS = {
            OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE
    };

    static File updatesDir(Context context) {
        File dir = new File(context.getCacheDir(), "updates");
        if (!dir.exists()) {
            //noinspection ResultOfMethodCallIgnored
            dir.mkdirs();
        }
        return dir;
    }

    static Uri uriFor(Context context, File file) {
        return new Uri.Builder()
                .scheme("content")
                .authority(context.getPackageName() + ".apkupdate")
                .appendPath(file.getName())
                .build();
    }

    private File resolveApk(Uri uri) throws FileNotFoundException {
        Context context = getContext();
        if (context == null || uri == null) {
            throw new FileNotFoundException("Missing update URI");
        }
        String name = uri.getLastPathSegment();
        if (name == null
                || name.isEmpty()
                || name.contains("/")
                || name.contains("\\")
                || name.contains("..")
                || !name.toLowerCase(Locale.US).endsWith(".apk")) {
            throw new FileNotFoundException("Invalid update APK name");
        }
        File dir = updatesDir(context);
        File target = new File(dir, name);
        try {
            if (Files.isSymbolicLink(target.toPath())) {
                throw new FileNotFoundException("Symlink update file rejected");
            }
            String dirCanon = dir.getCanonicalPath() + File.separator;
            String targetCanon = target.getCanonicalPath();
            if (!targetCanon.startsWith(dirCanon) || !target.isFile()) {
                throw new FileNotFoundException("Update APK not found");
            }
            return target;
        } catch (IOException e) {
            throw new FileNotFoundException("Unreadable update APK");
        }
    }

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public String getType(Uri uri) {
        return APK_MIME;
    }

    @Override
    public Cursor query(
            Uri uri,
            String[] projection,
            String selection,
            String[] selectionArgs,
            String sortOrder) {
        File target;
        try {
            target = resolveApk(uri);
        } catch (FileNotFoundException e) {
            return new MatrixCursor(DEFAULT_COLUMNS, 0);
        }
        String[] cols = (projection != null && projection.length > 0) ? projection : DEFAULT_COLUMNS;
        MatrixCursor cursor = new MatrixCursor(cols, 1);
        Object[] row = new Object[cols.length];
        for (int i = 0; i < cols.length; i++) {
            if (OpenableColumns.DISPLAY_NAME.equals(cols[i])) {
                row[i] = target.getName();
            } else if (OpenableColumns.SIZE.equals(cols[i])) {
                row[i] = target.length();
            }
        }
        cursor.addRow(row);
        return cursor;
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        if (!"r".equals(mode)) {
            throw new SecurityException("Update APKs are read-only");
        }
        File target = resolveApk(uri);
        return ParcelFileDescriptor.open(target, ParcelFileDescriptor.MODE_READ_ONLY);
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        throw new UnsupportedOperationException("Read-only provider");
    }

    @Override
    public int delete(Uri uri, String selection, String[] selectionArgs) {
        throw new UnsupportedOperationException("Read-only provider");
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] selectionArgs) {
        throw new UnsupportedOperationException("Read-only provider");
    }
}
