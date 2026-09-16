#Requires -Version 5.1
<#
  Faraday - identite Windows du lot applicatif.

  Probleme : en mode sandbox, faraday.exe est le bootstrap.exe de CEF renomme
  (icone CEF et nom "CEF Bootstrap Application") et faraday_helper.exe est son
  sous-processus (aucune icone, metadonnees vides). Explorer, le menu Demarrer,
  la barre des taches et les raccourcis affichent donc l'identite de CEF.

  Solution : copier les ressources Windows du module applicatif (faraday.dll, qui
  porte l'icone et les metadonnees de Faraday) vers ces executables :
    - RT_ICON + RT_GROUP_ICON  (icone)
    - RT_VERSION              (nom de produit, description, version)

  A executer AVANT la signature : toute modification d'un binaire invalide sa
  signature Authenticode.

  Usage :
    .\packaging\set-app-resources.ps1 -Stage dist\Faraday
    .\packaging\set-app-resources.ps1 -Stage dist\Faraday -Source faraday.dll `
        -Targets faraday.exe,faraday_helper.exe
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Stage,
    [string]$Source = "faraday.dll",
    [string[]]$Targets = @("faraday.exe", "faraday_helper.exe")
)

$ErrorActionPreference = "Stop"

$srcPath = if ([System.IO.Path]::IsPathRooted($Source)) { $Source } else { Join-Path $Stage $Source }
if (-not (Test-Path $srcPath)) {
    Write-Warning "Source introuvable : $srcPath - identite Windows non appliquee."
    return
}

# --- Outil en C# : acces direct aux API de ressources de Windows -------------
if (-not ("FaradayResources" -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class FaradayResources
{
    const uint LOAD_LIBRARY_AS_DATAFILE = 0x00000002;
    const int RT_ICON = 3;
    const int RT_GROUP_ICON = 14;
    const int RT_VERSION = 16;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr LoadLibraryEx(string lpFileName, IntPtr hFile, uint dwFlags);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool FreeLibrary(IntPtr hModule);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr FindResourceEx(IntPtr hModule, IntPtr lpType, IntPtr lpName, ushort wLanguage);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr LoadResource(IntPtr hModule, IntPtr hResInfo);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr LockResource(IntPtr hResData);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern uint SizeofResource(IntPtr hModule, IntPtr hResInfo);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr BeginUpdateResource(string pFileName, bool bDeleteExistingResources);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool UpdateResource(IntPtr hUpdate, IntPtr lpType, IntPtr lpName, ushort wLanguage, byte[] lpData, uint cbData);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool EndUpdateResource(IntPtr hUpdate, bool fDiscard);

    delegate bool EnumNameProc(IntPtr hModule, IntPtr lpType, IntPtr lpName, IntPtr lParam);
    delegate bool EnumLangProc(IntPtr hModule, IntPtr lpType, IntPtr lpName, ushort wLanguage, IntPtr lParam);

    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool EnumResourceNames(IntPtr hModule, IntPtr lpType, EnumNameProc lpEnumFunc, IntPtr lParam);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool EnumResourceLanguages(IntPtr hModule, IntPtr lpType, IntPtr lpName, EnumLangProc lpEnumFunc, IntPtr lParam);

    static List<IntPtr> Names(IntPtr h, int type)
    {
        var found = new List<IntPtr>();
        EnumNameProc cb = delegate(IntPtr m, IntPtr t, IntPtr n, IntPtr p) { found.Add(n); return true; };
        EnumResourceNames(h, (IntPtr)type, cb, IntPtr.Zero);
        return found;
    }

    static List<ushort> Langs(IntPtr h, int type, IntPtr name)
    {
        var found = new List<ushort>();
        EnumLangProc cb = delegate(IntPtr m, IntPtr t, IntPtr n, ushort l, IntPtr p) { found.Add(l); return true; };
        EnumResourceLanguages(h, (IntPtr)type, name, cb, IntPtr.Zero);
        if (found.Count == 0) found.Add(0);
        return found;
    }

    static byte[] Read(IntPtr h, int type, IntPtr name, ushort lang)
    {
        IntPtr info = FindResourceEx(h, (IntPtr)type, name, lang);
        if (info == IntPtr.Zero) return null;
        uint size = SizeofResource(h, info);
        if (size == 0) return null;
        IntPtr data = LoadResource(h, info);
        IntPtr ptr = LockResource(data);
        if (ptr == IntPtr.Zero) return null;
        byte[] buf = new byte[size];
        Marshal.Copy(ptr, buf, 0, (int)size);
        return buf;
    }

    static ushort IconId(byte[] group, int index)
    {
        // GRPICONDIR : 6 octets d'entete, puis des entrees de 14 octets ;
        // l'identifiant de l'image (nID) est le dernier mot de chaque entree.
        return (ushort)(group[6 + index * 14 + 12] | (group[6 + index * 14 + 13] << 8));
    }

    /// <summary>Copie icone + version de source vers target. Renvoie un compte rendu.</summary>
    public static string Transplant(string source, string target)
    {
        var rapport = new List<string>();
        IntPtr hSrc = LoadLibraryEx(source, IntPtr.Zero, LOAD_LIBRARY_AS_DATAFILE);
        if (hSrc == IntPtr.Zero)
            return "ECHEC : lecture impossible de " + source + " (code " + Marshal.GetLastWin32Error() + ")";

        // 1) Lire l'icone (groupe + images) et la version de la source.
        var groupes = new List<object[]>();   // { name, lang, blob, uint[] ids, byte[][] images }
        foreach (IntPtr gname in Names(hSrc, RT_GROUP_ICON))
        {
            foreach (ushort lang in Langs(hSrc, RT_GROUP_ICON, gname))
            {
                byte[] blob = Read(hSrc, RT_GROUP_ICON, gname, lang);
                if (blob == null || blob.Length < 6) continue;
                int count = blob[4] | (blob[5] << 8);
                var images = new List<byte[]>();
                var ids = new List<uint>();
                bool complet = true;
                for (int i = 0; i < count; i++)
                {
                    uint id = IconId(blob, i);
                    byte[] img = Read(hSrc, RT_ICON, (IntPtr)id, lang);
                    if (img == null) { complet = false; break; }
                    ids.Add(id);
                    images.Add(img);
                }
                if (!complet) continue;
                groupes.Add(new object[] { gname, lang, blob, ids.ToArray(), images.ToArray() });
            }
        }

        byte[] version = null;
        ushort versionLang = 0;
        var vnames = Names(hSrc, RT_VERSION);
        if (vnames.Count > 0)
        {
            var langs = Langs(hSrc, RT_VERSION, vnames[0]);
            versionLang = langs[0];
            version = Read(hSrc, RT_VERSION, vnames[0], versionLang);
        }
        FreeLibrary(hSrc);

        if (groupes.Count == 0 && version == null)
            return "ECHEC : aucune icone ni version lisible dans " + source;

        // 2) Ecrire dans la cible (remplace ce qui existe).
        IntPtr hUpd = BeginUpdateResource(target, false);
        if (hUpd == IntPtr.Zero)
            return "ECHEC : ecriture impossible dans " + target + " (code " + Marshal.GetLastWin32Error() + ")";

        // 2a) Supprimer les anciennes ressources d'icone / version de la cible.
        IntPtr hDst = LoadLibraryEx(target, IntPtr.Zero, LOAD_LIBRARY_AS_DATAFILE);
        if (hDst != IntPtr.Zero)
        {
            foreach (int type in new int[] { RT_ICON, RT_GROUP_ICON, RT_VERSION })
            {
                foreach (IntPtr name in Names(hDst, type))
                {
                    foreach (ushort lang in Langs(hDst, type, name))
                        UpdateResource(hUpd, (IntPtr)type, name, lang, null, 0);
                }
            }
            FreeLibrary(hDst);
        }

        // 2b) Ecrire les nouvelles ressources.
        int nbImages = 0;
        if (groupes.Count > 0)
        {
            object[] g = groupes[0];
            IntPtr gname = (IntPtr)g[0];
            ushort glang = (ushort)g[1];
            byte[] gblob = (byte[])g[2];
            uint[] ids = (uint[])g[3];
            byte[][] images = (byte[][])g[4];
            for (int i = 0; i < images.Length; i++)
            {
                if (!UpdateResource(hUpd, (IntPtr)RT_ICON, (IntPtr)ids[i], glang, images[i], (uint)images[i].Length))
                    rapport.Add("avertissement : image d'icone " + ids[i] + " non ecrite");
                else
                    nbImages++;
            }
            if (!UpdateResource(hUpd, (IntPtr)RT_GROUP_ICON, gname, glang, gblob, (uint)gblob.Length))
                rapport.Add("avertissement : groupe d'icones non ecrit");
        }
        if (version != null)
        {
            if (!UpdateResource(hUpd, (IntPtr)RT_VERSION, (IntPtr)1, versionLang, version, (uint)version.Length))
                rapport.Add("avertissement : metadonnees de version non ecrites");
        }

        if (!EndUpdateResource(hUpd, false))
            return "ECHEC : enregistrement des ressources refuse pour " + target;

        rapport.Insert(0, nbImages + " image(s) d'icone + metadonnees copiees");
        return string.Join(" / ", rapport.ToArray());
    }
}
'@
}

$echecs = 0
foreach ($t in $Targets) {
    $dstPath = Join-Path $Stage $t
    if (-not (Test-Path $dstPath)) {
        Write-Host ("==> {0,-20} absent, ignore" -f $t)
        continue
    }
    $avant = (Get-Item $dstPath).Length
    $res = [FaradayResources]::Transplant($srcPath, $dstPath)
    $apres = (Get-Item $dstPath).Length
    if ($res -like "ECHEC*") {
        $echecs++
        Write-Warning ("{0} : {1}" -f $t, $res)
    } else {
        Write-Host ("==> {0,-20} {1}  ({2:N0} -> {3:N0} octets)" -f $t, $res, $avant, $apres)
    }
}

if ($echecs -gt 0) { throw "Identite Windows : $echecs fichier(s) en echec." }
Write-Host "==> Identite Windows appliquee (icone + metadonnees) - a signer ensuite."
