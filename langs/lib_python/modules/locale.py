# Stub: the plain C locale is the only locale carried here.
LC_CTYPE = 0
LC_NUMERIC = 1
LC_TIME = 2
LC_COLLATE = 3
LC_MONETARY = 4
LC_MESSAGES = 5
LC_ALL = 6
CHAR_MAX = 127

def setlocale(category, locale=None):
    if category < 0 or category > 6:
        raise 'ValueError: invalid locale category'
    if locale is not None and locale != 'C' and locale != 'POSIX' and locale != '':
        raise 'NotImplementedError: locale.setlocale supports only C'
    return 'C'

def getlocale(category=0):
    if category == LC_ALL:
        raise 'TypeError: category LC_ALL is not supported'
    if category < 0 or category > 6:
        raise 'ValueError: invalid locale category'
    return (None, None)

def localeconv():
    return {'decimal_point': '.', 'thousands_sep': '', 'grouping': [], 'int_curr_symbol': '', 'currency_symbol': '', 'mon_decimal_point': '', 'mon_thousands_sep': '', 'mon_grouping': [], 'positive_sign': '', 'negative_sign': '', 'int_frac_digits': 127, 'frac_digits': 127, 'p_cs_precedes': 127, 'p_sep_by_space': 127, 'n_cs_precedes': 127, 'n_sep_by_space': 127, 'p_sign_posn': 127, 'n_sign_posn': 127}

# The name-alias engine and its two tables are CPython's Lib/locale.py at
# v3.14.8 / 8e6e75d9102e; the encoding-name normaliser it calls is
# CPython's Lib/encodings/__init__.py, carried here because the encodings
# package is not part of this run's library. Both alias tables the
# engine reads are carried as well: Lib/encodings/aliases.py, and
# Lib/locale.py's own locale_encoding_alias and locale_alias, each at
# v3.14.8 / 8e6e75d9102e.
_encoding_aliases = {

    # Please keep this list sorted alphabetically by value !

    # ascii codec
    '646'                : 'ascii',
    'ansi_x3.4_1968'     : 'ascii',
    'ansi_x3_4_1968'     : 'ascii', # some email headers use this non-standard name
    'ansi_x3.4_1986'     : 'ascii',
    'cp367'              : 'ascii',
    'csascii'            : 'ascii',
    'ibm367'             : 'ascii',
    'iso646_us'          : 'ascii',
    'iso_646.irv_1991'   : 'ascii',
    'iso_ir_6'           : 'ascii',
    'us'                 : 'ascii',
    'us_ascii'           : 'ascii',

    # base64_codec codec
    'base64'             : 'base64_codec',
    'base_64'            : 'base64_codec',

    # big5 codec
    'big5_tw'            : 'big5',
    'csbig5'             : 'big5',

    # big5hkscs codec
    'big5_hkscs'         : 'big5hkscs',
    'hkscs'              : 'big5hkscs',

    # bz2_codec codec
    'bz2'                : 'bz2_codec',

    # cp037 codec
    '037'                : 'cp037',
    'csibm037'           : 'cp037',
    'ebcdic_cp_ca'       : 'cp037',
    'ebcdic_cp_nl'       : 'cp037',
    'ebcdic_cp_us'       : 'cp037',
    'ebcdic_cp_wt'       : 'cp037',
    'ibm037'             : 'cp037',
    'ibm039'             : 'cp037',

    # cp1026 codec
    '1026'               : 'cp1026',
    'csibm1026'          : 'cp1026',
    'ibm1026'            : 'cp1026',

    # cp1125 codec
    '1125'                : 'cp1125',
    'ibm1125'             : 'cp1125',
    'cp866u'              : 'cp1125',
    'ruscii'              : 'cp1125',

    # cp1140 codec
    '1140'               : 'cp1140',
    'cp01140'            : 'cp1140',
    'csibm01140'         : 'cp1140',
    'ebcdic_us_37_euro'  : 'cp1140',
    'ibm01140'           : 'cp1140',
    'ibm1140'            : 'cp1140',

    # cp1250 codec
    '1250'               : 'cp1250',
    'windows_1250'       : 'cp1250',

    # cp1251 codec
    '1251'               : 'cp1251',
    'windows_1251'       : 'cp1251',

    # cp1252 codec
    '1252'               : 'cp1252',
    'windows_1252'       : 'cp1252',

    # cp1253 codec
    '1253'               : 'cp1253',
    'windows_1253'       : 'cp1253',

    # cp1254 codec
    '1254'               : 'cp1254',
    'windows_1254'       : 'cp1254',

    # cp1255 codec
    '1255'               : 'cp1255',
    'windows_1255'       : 'cp1255',

    # cp1256 codec
    '1256'               : 'cp1256',
    'windows_1256'       : 'cp1256',

    # cp1257 codec
    '1257'               : 'cp1257',
    'windows_1257'       : 'cp1257',

    # cp1258 codec
    '1258'               : 'cp1258',
    'windows_1258'       : 'cp1258',

    # cp273 codec
    '273'                : 'cp273',
    'ibm273'             : 'cp273',
    'csibm273'           : 'cp273',

    # cp424 codec
    '424'                : 'cp424',
    'csibm424'           : 'cp424',
    'ebcdic_cp_he'       : 'cp424',
    'ibm424'             : 'cp424',

    # cp437 codec
    '437'                : 'cp437',
    'cspc8codepage437'   : 'cp437',
    'ibm437'             : 'cp437',

    # cp500 codec
    '500'                : 'cp500',
    'csibm500'           : 'cp500',
    'ebcdic_cp_be'       : 'cp500',
    'ebcdic_cp_ch'       : 'cp500',
    'ibm500'             : 'cp500',

    # cp775 codec
    '775'                : 'cp775',
    'cspc775baltic'      : 'cp775',
    'ibm775'             : 'cp775',

    # cp850 codec
    '850'                : 'cp850',
    'cspc850multilingual' : 'cp850',
    'ibm850'             : 'cp850',

    # cp852 codec
    '852'                : 'cp852',
    'cspcp852'           : 'cp852',
    'ibm852'             : 'cp852',

    # cp855 codec
    '855'                : 'cp855',
    'csibm855'           : 'cp855',
    'ibm855'             : 'cp855',

    # cp857 codec
    '857'                : 'cp857',
    'csibm857'           : 'cp857',
    'ibm857'             : 'cp857',

    # cp858 codec
    '858'                : 'cp858',
    'cp00858'            : 'cp858',
    'csibm00858'         : 'cp858',
    'csibm858'           : 'cp858',
    'ibm00858'           : 'cp858',
    'ibm858'             : 'cp858',
    'pc_multilingual_850_euro' : 'cp858',

    # cp860 codec
    '860'                : 'cp860',
    'csibm860'           : 'cp860',
    'ibm860'             : 'cp860',

    # cp861 codec
    '861'                : 'cp861',
    'cp_is'              : 'cp861',
    'csibm861'           : 'cp861',
    'ibm861'             : 'cp861',

    # cp862 codec
    '862'                : 'cp862',
    'cspc862latinhebrew' : 'cp862',
    'ibm862'             : 'cp862',

    # cp863 codec
    '863'                : 'cp863',
    'csibm863'           : 'cp863',
    'ibm863'             : 'cp863',

    # cp864 codec
    '864'                : 'cp864',
    'csibm864'           : 'cp864',
    'ibm864'             : 'cp864',

    # cp865 codec
    '865'                : 'cp865',
    'csibm865'           : 'cp865',
    'ibm865'             : 'cp865',

    # cp866 codec
    '866'                : 'cp866',
    'csibm866'           : 'cp866',
    'ibm866'             : 'cp866',

    # cp869 codec
    '869'                : 'cp869',
    'cp_gr'              : 'cp869',
    'csibm869'           : 'cp869',
    'ibm869'             : 'cp869',

    # cp874 codec
    '874'                : 'cp874',
    'ms874'              : 'cp874',
    'windows_874'        : 'cp874',

    # cp932 codec
    '932'                : 'cp932',
    'ms932'              : 'cp932',
    'mskanji'            : 'cp932',
    'ms_kanji'           : 'cp932',
    'windows_31j'        : 'cp932',

    # cp949 codec
    '949'                : 'cp949',
    'ms949'              : 'cp949',
    'uhc'                : 'cp949',

    # cp950 codec
    '950'                : 'cp950',
    'ms950'              : 'cp950',

    # euc_jis_2004 codec
    'jisx0213'           : 'euc_jis_2004',
    'eucjis2004'         : 'euc_jis_2004',
    'euc_jis2004'        : 'euc_jis_2004',

    # euc_jisx0213 codec
    'eucjisx0213'        : 'euc_jisx0213',

    # euc_jp codec
    'eucjp'              : 'euc_jp',
    'ujis'               : 'euc_jp',
    'u_jis'              : 'euc_jp',

    # euc_kr codec
    'euckr'              : 'euc_kr',
    'korean'             : 'euc_kr',
    'ksc5601'            : 'euc_kr',
    'ks_c_5601'          : 'euc_kr',
    'ks_c_5601_1987'     : 'euc_kr',
    'ksx1001'            : 'euc_kr',
    'ks_x_1001'          : 'euc_kr',
    'cseuckr'            : 'euc_kr',

    # gb18030 codec
    'gb18030_2000'       : 'gb18030',

    # gb2312 codec
    'chinese'            : 'gb2312',
    'csiso58gb231280'    : 'gb2312',
    'euc_cn'             : 'gb2312',
    'euccn'              : 'gb2312',
    'eucgb2312_cn'       : 'gb2312',
    'gb2312_1980'        : 'gb2312',
    'gb2312_80'          : 'gb2312',
    'iso_ir_58'          : 'gb2312',

    # gbk codec
    '936'                : 'gbk',
    'cp936'              : 'gbk',
    'ms936'              : 'gbk',

    # hex_codec codec
    'hex'                : 'hex_codec',

    # hp_roman8 codec
    'roman8'             : 'hp_roman8',
    'r8'                 : 'hp_roman8',
    'csHPRoman8'         : 'hp_roman8',
    'cp1051'             : 'hp_roman8',
    'ibm1051'            : 'hp_roman8',

    # hz codec
    'hzgb'               : 'hz',
    'hz_gb'              : 'hz',
    'hz_gb_2312'         : 'hz',

    # iso2022_jp codec
    'csiso2022jp'        : 'iso2022_jp',
    'iso2022jp'          : 'iso2022_jp',
    'iso_2022_jp'        : 'iso2022_jp',

    # iso2022_jp_1 codec
    'iso2022jp_1'        : 'iso2022_jp_1',
    'iso_2022_jp_1'      : 'iso2022_jp_1',

    # iso2022_jp_2 codec
    'iso2022jp_2'        : 'iso2022_jp_2',
    'iso_2022_jp_2'      : 'iso2022_jp_2',

    # iso2022_jp_2004 codec
    'iso_2022_jp_2004'   : 'iso2022_jp_2004',
    'iso2022jp_2004'     : 'iso2022_jp_2004',

    # iso2022_jp_3 codec
    'iso2022jp_3'        : 'iso2022_jp_3',
    'iso_2022_jp_3'      : 'iso2022_jp_3',

    # iso2022_jp_ext codec
    'iso2022jp_ext'      : 'iso2022_jp_ext',
    'iso_2022_jp_ext'    : 'iso2022_jp_ext',

    # iso2022_kr codec
    'csiso2022kr'        : 'iso2022_kr',
    'iso2022kr'          : 'iso2022_kr',
    'iso_2022_kr'        : 'iso2022_kr',

    # iso8859_10 codec
    'csisolatin6'        : 'iso8859_10',
    'iso_8859_10'        : 'iso8859_10',
    'iso_8859_10_1992'   : 'iso8859_10',
    'iso_ir_157'         : 'iso8859_10',
    'l6'                 : 'iso8859_10',
    'latin6'             : 'iso8859_10',

    # iso8859_11 codec
    'thai'               : 'iso8859_11',
    'iso_8859_11'        : 'iso8859_11',
    'iso_8859_11_2001'   : 'iso8859_11',

    # iso8859_13 codec
    'iso_8859_13'        : 'iso8859_13',
    'l7'                 : 'iso8859_13',
    'latin7'             : 'iso8859_13',

    # iso8859_14 codec
    'iso_8859_14'        : 'iso8859_14',
    'iso_8859_14_1998'   : 'iso8859_14',
    'iso_celtic'         : 'iso8859_14',
    'iso_ir_199'         : 'iso8859_14',
    'l8'                 : 'iso8859_14',
    'latin8'             : 'iso8859_14',

    # iso8859_15 codec
    'iso_8859_15'        : 'iso8859_15',
    'l9'                 : 'iso8859_15',
    'latin9'             : 'iso8859_15',

    # iso8859_16 codec
    'iso_8859_16'        : 'iso8859_16',
    'iso_8859_16_2001'   : 'iso8859_16',
    'iso_ir_226'         : 'iso8859_16',
    'l10'                : 'iso8859_16',
    'latin10'            : 'iso8859_16',

    # iso8859_2 codec
    'csisolatin2'        : 'iso8859_2',
    'iso_8859_2'         : 'iso8859_2',
    'iso_8859_2_1987'    : 'iso8859_2',
    'iso_ir_101'         : 'iso8859_2',
    'l2'                 : 'iso8859_2',
    'latin2'             : 'iso8859_2',

    # iso8859_3 codec
    'csisolatin3'        : 'iso8859_3',
    'iso_8859_3'         : 'iso8859_3',
    'iso_8859_3_1988'    : 'iso8859_3',
    'iso_ir_109'         : 'iso8859_3',
    'l3'                 : 'iso8859_3',
    'latin3'             : 'iso8859_3',

    # iso8859_4 codec
    'csisolatin4'        : 'iso8859_4',
    'iso_8859_4'         : 'iso8859_4',
    'iso_8859_4_1988'    : 'iso8859_4',
    'iso_ir_110'         : 'iso8859_4',
    'l4'                 : 'iso8859_4',
    'latin4'             : 'iso8859_4',

    # iso8859_5 codec
    'csisolatincyrillic' : 'iso8859_5',
    'cyrillic'           : 'iso8859_5',
    'iso_8859_5'         : 'iso8859_5',
    'iso_8859_5_1988'    : 'iso8859_5',
    'iso_ir_144'         : 'iso8859_5',

    # iso8859_6 codec
    'arabic'             : 'iso8859_6',
    'asmo_708'           : 'iso8859_6',
    'csisolatinarabic'   : 'iso8859_6',
    'ecma_114'           : 'iso8859_6',
    'iso_8859_6'         : 'iso8859_6',
    'iso_8859_6_1987'    : 'iso8859_6',
    'iso_ir_127'         : 'iso8859_6',

    # iso8859_7 codec
    'csisolatingreek'    : 'iso8859_7',
    'ecma_118'           : 'iso8859_7',
    'elot_928'           : 'iso8859_7',
    'greek'              : 'iso8859_7',
    'greek8'             : 'iso8859_7',
    'iso_8859_7'         : 'iso8859_7',
    'iso_8859_7_1987'    : 'iso8859_7',
    'iso_ir_126'         : 'iso8859_7',

    # iso8859_8 codec
    'csisolatinhebrew'   : 'iso8859_8',
    'hebrew'             : 'iso8859_8',
    'iso_8859_8'         : 'iso8859_8',
    'iso_8859_8_1988'    : 'iso8859_8',
    'iso_ir_138'         : 'iso8859_8',
    'iso_8859_8_i'       : 'iso8859_8',
    'iso_8859_8_e'       : 'iso8859_8',

    # iso8859_9 codec
    'csisolatin5'        : 'iso8859_9',
    'iso_8859_9'         : 'iso8859_9',
    'iso_8859_9_1989'    : 'iso8859_9',
    'iso_ir_148'         : 'iso8859_9',
    'l5'                 : 'iso8859_9',
    'latin5'             : 'iso8859_9',

    # johab codec
    'cp1361'             : 'johab',
    'ms1361'             : 'johab',

    # koi8_r codec
    'cskoi8r'            : 'koi8_r',

    # kz1048 codec
    'kz_1048'           : 'kz1048',
    'rk1048'            : 'kz1048',
    'strk1048_2002'     : 'kz1048',

    # latin_1 codec
    #
    # Note that the latin_1 codec is implemented internally in C and a
    # lot faster than the charmap codec iso8859_1 which uses the same
    # encoding. This is why we discourage the use of the iso8859_1
    # codec and alias it to latin_1 instead.
    #
    '8859'               : 'latin_1',
    'cp819'              : 'latin_1',
    'csisolatin1'        : 'latin_1',
    'ibm819'             : 'latin_1',
    'iso8859'            : 'latin_1',
    'iso8859_1'          : 'latin_1',
    'iso_8859_1'         : 'latin_1',
    'iso_8859_1_1987'    : 'latin_1',
    'iso_ir_100'         : 'latin_1',
    'l1'                 : 'latin_1',
    'latin'              : 'latin_1',
    'latin1'             : 'latin_1',

    # mac_cyrillic codec
    'maccyrillic'        : 'mac_cyrillic',

    # mac_greek codec
    'macgreek'           : 'mac_greek',

    # mac_iceland codec
    'maciceland'         : 'mac_iceland',

    # mac_latin2 codec
    'maccentraleurope'   : 'mac_latin2',
    'mac_centeuro'       : 'mac_latin2',
    'maclatin2'          : 'mac_latin2',

    # mac_roman codec
    'macintosh'          : 'mac_roman',
    'macroman'           : 'mac_roman',

    # mac_turkish codec
    'macturkish'         : 'mac_turkish',

    # mbcs codec
    'ansi'               : 'mbcs',
    'dbcs'               : 'mbcs',

    # ptcp154 codec
    'csptcp154'          : 'ptcp154',
    'pt154'              : 'ptcp154',
    'cp154'              : 'ptcp154',
    'cyrillic_asian'     : 'ptcp154',

    # quopri_codec codec
    'quopri'             : 'quopri_codec',
    'quoted_printable'   : 'quopri_codec',
    'quotedprintable'    : 'quopri_codec',

    # rot_13 codec
    'rot13'              : 'rot_13',

    # shift_jis codec
    'csshiftjis'         : 'shift_jis',
    'shiftjis'           : 'shift_jis',
    'sjis'               : 'shift_jis',
    's_jis'              : 'shift_jis',

    # shift_jis_2004 codec
    'shiftjis2004'       : 'shift_jis_2004',
    'sjis_2004'          : 'shift_jis_2004',
    's_jis_2004'         : 'shift_jis_2004',

    # shift_jisx0213 codec
    'shiftjisx0213'      : 'shift_jisx0213',
    'sjisx0213'          : 'shift_jisx0213',
    's_jisx0213'         : 'shift_jisx0213',

    # tis_620 codec
    'tis620'             : 'tis_620',
    'tis_620_0'          : 'tis_620',
    'tis_620_2529_0'     : 'tis_620',
    'tis_620_2529_1'     : 'tis_620',
    'iso_ir_166'         : 'tis_620',

    # utf_16 codec
    'u16'                : 'utf_16',
    'utf16'              : 'utf_16',

    # utf_16_be codec
    'unicodebigunmarked' : 'utf_16_be',
    'utf_16be'           : 'utf_16_be',

    # utf_16_le codec
    'unicodelittleunmarked' : 'utf_16_le',
    'utf_16le'           : 'utf_16_le',

    # utf_32 codec
    'u32'                : 'utf_32',
    'utf32'              : 'utf_32',

    # utf_32_be codec
    'utf_32be'           : 'utf_32_be',

    # utf_32_le codec
    'utf_32le'           : 'utf_32_le',

    # utf_7 codec
    'u7'                 : 'utf_7',
    'utf7'               : 'utf_7',
    'unicode_1_1_utf_7'  : 'utf_7',

    # utf_8 codec
    'u8'                 : 'utf_8',
    'utf'                : 'utf_8',
    'utf8'               : 'utf_8',
    'utf8_ucs2'          : 'utf_8',
    'utf8_ucs4'          : 'utf_8',
    'cp65001'            : 'utf_8',

    # uu_codec codec
    'uu'                 : 'uu_codec',

    # zlib_codec codec
    'zip'                : 'zlib_codec',
    'zlib'               : 'zlib_codec',

    # temporary mac CJK aliases, will be replaced by proper codecs in 3.1
    'x_mac_japanese'      : 'shift_jis',
    'x_mac_korean'        : 'euc_kr',
    'x_mac_simp_chinese'  : 'gb2312',
    'x_mac_trad_chinese'  : 'big5',
}


def normalize_encoding(encoding):

    """ Normalize an encoding name.

        Normalization works as follows: all non-alphanumeric
        characters except the dot used for Python package names are
        collapsed and replaced with a single underscore, e.g. '  -;#'
        becomes '_'. Leading and trailing underscores are removed.

        Note that encoding names should be ASCII only.

    """
    if isinstance(encoding, bytes):
        encoding = str(encoding, "ascii")

    chars = []
    punct = False
    for c in encoding:
        if c.isalnum() or c == '.':
            if punct and chars:
                chars.append('_')
            if c.isascii():
                chars.append(c)
            punct = False
        else:
            punct = True
    return ''.join(chars)



locale_encoding_alias = {

    # Mappings for non-standard encoding names used in locale names
    '437':                          'C',
    'c':                            'C',
    'en':                           'ISO8859-1',
    'jis':                          'JIS7',
    'jis7':                         'JIS7',
    'ajec':                         'eucJP',
    'koi8c':                        'KOI8-C',
    'microsoftcp1251':              'CP1251',
    'microsoftcp1255':              'CP1255',
    'microsoftcp1256':              'CP1256',
    '88591':                        'ISO8859-1',
    '88592':                        'ISO8859-2',
    '88595':                        'ISO8859-5',
    '885915':                       'ISO8859-15',

    # Mappings from Python codec names to C lib encoding names
    'ascii':                        'ISO8859-1',
    'latin_1':                      'ISO8859-1',
    'iso8859_1':                    'ISO8859-1',
    'iso8859_10':                   'ISO8859-10',
    'iso8859_11':                   'ISO8859-11',
    'iso8859_13':                   'ISO8859-13',
    'iso8859_14':                   'ISO8859-14',
    'iso8859_15':                   'ISO8859-15',
    'iso8859_16':                   'ISO8859-16',
    'iso8859_2':                    'ISO8859-2',
    'iso8859_3':                    'ISO8859-3',
    'iso8859_4':                    'ISO8859-4',
    'iso8859_5':                    'ISO8859-5',
    'iso8859_6':                    'ISO8859-6',
    'iso8859_7':                    'ISO8859-7',
    'iso8859_8':                    'ISO8859-8',
    'iso8859_9':                    'ISO8859-9',
    'iso2022_jp':                   'JIS7',
    'shift_jis':                    'SJIS',
    'tactis':                       'TACTIS',
    'euc_jp':                       'eucJP',
    'euc_kr':                       'eucKR',
    'utf_8':                        'UTF-8',
    'koi8_r':                       'KOI8-R',
    'koi8_t':                       'KOI8-T',
    'koi8_u':                       'KOI8-U',
    'kz1048':                       'RK1048',
    'cp1251':                       'CP1251',
    'cp1255':                       'CP1255',
    'cp1256':                       'CP1256',

    # XXX This list is still incomplete. If you know more
    # mappings, please file a bug report. Thanks.
}


locale_alias = {
    'a3':                                   'az_AZ.KOI8-C',
    'a3_az':                                'az_AZ.KOI8-C',
    'a3_az.koic':                           'az_AZ.KOI8-C',
    'aa_dj':                                'aa_DJ.ISO8859-1',
    'aa_er':                                'aa_ER.UTF-8',
    'aa_et':                                'aa_ET.UTF-8',
    'af':                                   'af_ZA.ISO8859-1',
    'af_za':                                'af_ZA.ISO8859-1',
    'agr_pe':                               'agr_PE.UTF-8',
    'ak_gh':                                'ak_GH.UTF-8',
    'am':                                   'am_ET.UTF-8',
    'am_et':                                'am_ET.UTF-8',
    'american':                             'en_US.ISO8859-1',
    'an_es':                                'an_ES.ISO8859-15',
    'anp_in':                               'anp_IN.UTF-8',
    'ar':                                   'ar_AA.ISO8859-6',
    'ar_aa':                                'ar_AA.ISO8859-6',
    'ar_ae':                                'ar_AE.ISO8859-6',
    'ar_bh':                                'ar_BH.ISO8859-6',
    'ar_dz':                                'ar_DZ.ISO8859-6',
    'ar_eg':                                'ar_EG.ISO8859-6',
    'ar_in':                                'ar_IN.UTF-8',
    'ar_iq':                                'ar_IQ.ISO8859-6',
    'ar_jo':                                'ar_JO.ISO8859-6',
    'ar_kw':                                'ar_KW.ISO8859-6',
    'ar_lb':                                'ar_LB.ISO8859-6',
    'ar_ly':                                'ar_LY.ISO8859-6',
    'ar_ma':                                'ar_MA.ISO8859-6',
    'ar_om':                                'ar_OM.ISO8859-6',
    'ar_qa':                                'ar_QA.ISO8859-6',
    'ar_sa':                                'ar_SA.ISO8859-6',
    'ar_sd':                                'ar_SD.ISO8859-6',
    'ar_ss':                                'ar_SS.UTF-8',
    'ar_sy':                                'ar_SY.ISO8859-6',
    'ar_tn':                                'ar_TN.ISO8859-6',
    'ar_ye':                                'ar_YE.ISO8859-6',
    'arabic':                               'ar_AA.ISO8859-6',
    'as':                                   'as_IN.UTF-8',
    'as_in':                                'as_IN.UTF-8',
    'ast_es':                               'ast_ES.ISO8859-15',
    'ayc_pe':                               'ayc_PE.UTF-8',
    'az':                                   'az_AZ.ISO8859-9E',
    'az_az':                                'az_AZ.ISO8859-9E',
    'az_az.iso88599e':                      'az_AZ.ISO8859-9E',
    'az_ir':                                'az_IR.UTF-8',
    'be':                                   'be_BY.CP1251',
    'be@latin':                             'be_BY.UTF-8@latin',
    'be_bg.utf8':                           'bg_BG.UTF-8',
    'be_by':                                'be_BY.CP1251',
    'be_by@latin':                          'be_BY.UTF-8@latin',
    'bem_zm':                               'bem_ZM.UTF-8',
    'ber_dz':                               'ber_DZ.UTF-8',
    'ber_ma':                               'ber_MA.UTF-8',
    'bg':                                   'bg_BG.CP1251',
    'bg_bg':                                'bg_BG.CP1251',
    'bhb_in.utf8':                          'bhb_IN.UTF-8',
    'bho_in':                               'bho_IN.UTF-8',
    'bho_np':                               'bho_NP.UTF-8',
    'bi_vu':                                'bi_VU.UTF-8',
    'bn_bd':                                'bn_BD.UTF-8',
    'bn_in':                                'bn_IN.UTF-8',
    'bo_cn':                                'bo_CN.UTF-8',
    'bo_in':                                'bo_IN.UTF-8',
    'bokmal':                               'nb_NO.ISO8859-1',
    'bokm\xe5l':                            'nb_NO.ISO8859-1',
    'br':                                   'br_FR.ISO8859-1',
    'br_fr':                                'br_FR.ISO8859-1',
    'brx_in':                               'brx_IN.UTF-8',
    'bs':                                   'bs_BA.ISO8859-2',
    'bs_ba':                                'bs_BA.ISO8859-2',
    'bulgarian':                            'bg_BG.CP1251',
    'byn_er':                               'byn_ER.UTF-8',
    'c':                                    'C',
    'c-french':                             'fr_CA.ISO8859-1',
    'c.ascii':                              'C',
    'c.en':                                 'C',
    'c.iso88591':                           'en_US.ISO8859-1',
    'c_c':                                  'C',
    'c_c.c':                                'C',
    'ca':                                   'ca_ES.ISO8859-1',
    'ca_ad':                                'ca_AD.ISO8859-1',
    'ca_es':                                'ca_ES.ISO8859-1',
    'ca_es@valencia':                       'ca_ES.UTF-8@valencia',
    'ca_fr':                                'ca_FR.ISO8859-1',
    'ca_it':                                'ca_IT.ISO8859-1',
    'catalan':                              'ca_ES.ISO8859-1',
    'ce_ru':                                'ce_RU.UTF-8',
    'cextend':                              'en_US.ISO8859-1',
    'chinese-s':                            'zh_CN.eucCN',
    'chinese-t':                            'zh_TW.eucTW',
    'chr_us':                               'chr_US.UTF-8',
    'ckb_iq':                               'ckb_IQ.UTF-8',
    'cmn_tw':                               'cmn_TW.UTF-8',
    'crh_ru':                               'crh_RU.UTF-8',
    'crh_ua':                               'crh_UA.UTF-8',
    'croatian':                             'hr_HR.ISO8859-2',
    'cs':                                   'cs_CZ.ISO8859-2',
    'cs_cs':                                'cs_CZ.ISO8859-2',
    'cs_cz':                                'cs_CZ.ISO8859-2',
    'csb_pl':                               'csb_PL.UTF-8',
    'cv_ru':                                'cv_RU.UTF-8',
    'cy':                                   'cy_GB.ISO8859-1',
    'cy_gb':                                'cy_GB.ISO8859-1',
    'cz':                                   'cs_CZ.ISO8859-2',
    'cz_cz':                                'cs_CZ.ISO8859-2',
    'czech':                                'cs_CZ.ISO8859-2',
    'da':                                   'da_DK.ISO8859-1',
    'da_dk':                                'da_DK.ISO8859-1',
    'danish':                               'da_DK.ISO8859-1',
    'dansk':                                'da_DK.ISO8859-1',
    'de':                                   'de_DE.ISO8859-1',
    'de_at':                                'de_AT.ISO8859-1',
    'de_be':                                'de_BE.ISO8859-1',
    'de_ch':                                'de_CH.ISO8859-1',
    'de_de':                                'de_DE.ISO8859-1',
    'de_it':                                'de_IT.UTF-8',
    'de_li':                                'de_LI.ISO8859-1',
    'de_lu':                                'de_LU.ISO8859-1',
    'deutsch':                              'de_DE.ISO8859-1',
    'doi_in':                               'doi_IN.UTF-8',
    'dsb_de':                               'dsb_DE.UTF-8',
    'dutch':                                'nl_NL.ISO8859-1',
    'dutch.iso88591':                       'nl_BE.ISO8859-1',
    'dv_mv':                                'dv_MV.UTF-8',
    'dz_bt':                                'dz_BT.UTF-8',
    'ee':                                   'ee_EE.ISO8859-4',
    'ee_ee':                                'ee_EE.ISO8859-4',
    'eesti':                                'et_EE.ISO8859-1',
    'el':                                   'el_GR.ISO8859-7',
    'el_cy':                                'el_CY.ISO8859-7',
    'el_gr':                                'el_GR.ISO8859-7',
    'el_gr@euro':                           'el_GR.ISO8859-15',
    'en':                                   'en_US.ISO8859-1',
    'en_ag':                                'en_AG.UTF-8',
    'en_au':                                'en_AU.ISO8859-1',
    'en_be':                                'en_BE.ISO8859-1',
    'en_bw':                                'en_BW.ISO8859-1',
    'en_ca':                                'en_CA.ISO8859-1',
    'en_dk':                                'en_DK.ISO8859-1',
    'en_dl.utf8':                           'en_DL.UTF-8',
    'en_gb':                                'en_GB.ISO8859-1',
    'en_hk':                                'en_HK.ISO8859-1',
    'en_ie':                                'en_IE.ISO8859-1',
    'en_il':                                'en_IL.ISO8859-1',
    'en_in':                                'en_IN.ISO8859-1',
    'en_ng':                                'en_NG.UTF-8',
    'en_nz':                                'en_NZ.ISO8859-1',
    'en_ph':                                'en_PH.ISO8859-1',
    'en_sc.utf8':                           'en_SC.UTF-8',
    'en_sg':                                'en_SG.ISO8859-1',
    'en_uk':                                'en_GB.ISO8859-1',
    'en_us':                                'en_US.ISO8859-1',
    'en_us@euro@euro':                      'en_US.ISO8859-15',
    'en_za':                                'en_ZA.ISO8859-1',
    'en_zm':                                'en_ZM.UTF-8',
    'en_zw':                                'en_ZW.ISO8859-1',
    'en_zw.utf8':                           'en_ZS.UTF-8',
    'eng_gb':                               'en_GB.ISO8859-1',
    'english':                              'en_EN.ISO8859-1',
    'english_uk':                           'en_GB.ISO8859-1',
    'english_united-states':                'en_US.ISO8859-1',
    'english_united-states.437':            'C',
    'english_us':                           'en_US.ISO8859-1',
    'eo':                                   'eo_XX.ISO8859-3',
    'eo.utf8':                              'eo.UTF-8',
    'eo_eo':                                'eo_EO.ISO8859-3',
    'eo_us.utf8':                           'eo_US.UTF-8',
    'eo_xx':                                'eo_XX.ISO8859-3',
    'es':                                   'es_ES.ISO8859-1',
    'es_ar':                                'es_AR.ISO8859-1',
    'es_bo':                                'es_BO.ISO8859-1',
    'es_cl':                                'es_CL.ISO8859-1',
    'es_co':                                'es_CO.ISO8859-1',
    'es_cr':                                'es_CR.ISO8859-1',
    'es_cu':                                'es_CU.ISO8859-1',
    'es_do':                                'es_DO.ISO8859-1',
    'es_ec':                                'es_EC.ISO8859-1',
    'es_es':                                'es_ES.ISO8859-1',
    'es_gt':                                'es_GT.ISO8859-1',
    'es_hn':                                'es_HN.ISO8859-1',
    'es_mx':                                'es_MX.ISO8859-1',
    'es_ni':                                'es_NI.ISO8859-1',
    'es_pa':                                'es_PA.ISO8859-1',
    'es_pe':                                'es_PE.ISO8859-1',
    'es_pr':                                'es_PR.ISO8859-1',
    'es_py':                                'es_PY.ISO8859-1',
    'es_sv':                                'es_SV.ISO8859-1',
    'es_us':                                'es_US.ISO8859-1',
    'es_uy':                                'es_UY.ISO8859-1',
    'es_ve':                                'es_VE.ISO8859-1',
    'estonian':                             'et_EE.ISO8859-1',
    'et':                                   'et_EE.ISO8859-15',
    'et_ee':                                'et_EE.ISO8859-15',
    'eu':                                   'eu_ES.ISO8859-1',
    'eu_es':                                'eu_ES.ISO8859-1',
    'eu_fr':                                'eu_FR.ISO8859-1',
    'fa':                                   'fa_IR.UTF-8',
    'fa_ir':                                'fa_IR.UTF-8',
    'fa_ir.isiri3342':                      'fa_IR.ISIRI-3342',
    'ff_sn':                                'ff_SN.UTF-8',
    'fi':                                   'fi_FI.ISO8859-15',
    'fi_fi':                                'fi_FI.ISO8859-15',
    'fil_ph':                               'fil_PH.UTF-8',
    'finnish':                              'fi_FI.ISO8859-1',
    'fo':                                   'fo_FO.ISO8859-1',
    'fo_fo':                                'fo_FO.ISO8859-1',
    'fr':                                   'fr_FR.ISO8859-1',
    'fr_be':                                'fr_BE.ISO8859-1',
    'fr_ca':                                'fr_CA.ISO8859-1',
    'fr_ch':                                'fr_CH.ISO8859-1',
    'fr_fr':                                'fr_FR.ISO8859-1',
    'fr_lu':                                'fr_LU.ISO8859-1',
    'fran\xe7ais':                          'fr_FR.ISO8859-1',
    'fre_fr':                               'fr_FR.ISO8859-1',
    'french':                               'fr_FR.ISO8859-1',
    'french.iso88591':                      'fr_CH.ISO8859-1',
    'french_france':                        'fr_FR.ISO8859-1',
    'fur_it':                               'fur_IT.UTF-8',
    'fy_de':                                'fy_DE.UTF-8',
    'fy_nl':                                'fy_NL.UTF-8',
    'ga':                                   'ga_IE.ISO8859-1',
    'ga_ie':                                'ga_IE.ISO8859-1',
    'galego':                               'gl_ES.ISO8859-1',
    'galician':                             'gl_ES.ISO8859-1',
    'gbm_in':                               'gbm_IN.UTF-8',
    'gd':                                   'gd_GB.ISO8859-1',
    'gd_gb':                                'gd_GB.ISO8859-1',
    'ger_de':                               'de_DE.ISO8859-1',
    'german':                               'de_DE.ISO8859-1',
    'german.iso88591':                      'de_CH.ISO8859-1',
    'german_germany':                       'de_DE.ISO8859-1',
    'gez_er':                               'gez_ER.UTF-8',
    'gez_et':                               'gez_ET.UTF-8',
    'gl':                                   'gl_ES.ISO8859-1',
    'gl_es':                                'gl_ES.ISO8859-1',
    'greek':                                'el_GR.ISO8859-7',
    'gu_in':                                'gu_IN.UTF-8',
    'gv':                                   'gv_GB.ISO8859-1',
    'gv_gb':                                'gv_GB.ISO8859-1',
    'ha_ng':                                'ha_NG.UTF-8',
    'hak_tw':                               'hak_TW.UTF-8',
    'he':                                   'he_IL.ISO8859-8',
    'he_il':                                'he_IL.ISO8859-8',
    'hebrew':                               'he_IL.ISO8859-8',
    'hi':                                   'hi_IN.ISCII-DEV',
    'hi_in':                                'hi_IN.ISCII-DEV',
    'hi_in.isciidev':                       'hi_IN.ISCII-DEV',
    'hif_fj':                               'hif_FJ.UTF-8',
    'hne':                                  'hne_IN.UTF-8',
    'hne_in':                               'hne_IN.UTF-8',
    'hr':                                   'hr_HR.ISO8859-2',
    'hr_hr':                                'hr_HR.ISO8859-2',
    'hrvatski':                             'hr_HR.ISO8859-2',
    'hsb_de':                               'hsb_DE.ISO8859-2',
    'ht_ht':                                'ht_HT.UTF-8',
    'hu':                                   'hu_HU.ISO8859-2',
    'hu_hu':                                'hu_HU.ISO8859-2',
    'hungarian':                            'hu_HU.ISO8859-2',
    'hy_am':                                'hy_AM.UTF-8',
    'hy_am.armscii8':                       'hy_AM.ARMSCII_8',
    'ia':                                   'ia.UTF-8',
    'ia_fr':                                'ia_FR.UTF-8',
    'icelandic':                            'is_IS.ISO8859-1',
    'id':                                   'id_ID.ISO8859-1',
    'id_id':                                'id_ID.ISO8859-1',
    'ie':                                   'ie.UTF-8',
    'ig_ng':                                'ig_NG.UTF-8',
    'ik_ca':                                'ik_CA.UTF-8',
    'in':                                   'id_ID.ISO8859-1',
    'in_id':                                'id_ID.ISO8859-1',
    'is':                                   'is_IS.ISO8859-1',
    'is_is':                                'is_IS.ISO8859-1',
    'iso-8859-1':                           'en_US.ISO8859-1',
    'iso-8859-15':                          'en_US.ISO8859-15',
    'iso8859-1':                            'en_US.ISO8859-1',
    'iso8859-15':                           'en_US.ISO8859-15',
    'iso_8859_1':                           'en_US.ISO8859-1',
    'iso_8859_15':                          'en_US.ISO8859-15',
    'it':                                   'it_IT.ISO8859-1',
    'it_ch':                                'it_CH.ISO8859-1',
    'it_it':                                'it_IT.ISO8859-1',
    'italian':                              'it_IT.ISO8859-1',
    'iu':                                   'iu_CA.NUNACOM-8',
    'iu_ca':                                'iu_CA.NUNACOM-8',
    'iu_ca.nunacom8':                       'iu_CA.NUNACOM-8',
    'iw':                                   'he_IL.ISO8859-8',
    'iw_il':                                'he_IL.ISO8859-8',
    'iw_il.utf8':                           'iw_IL.UTF-8',
    'ja':                                   'ja_JP.eucJP',
    'ja_jp':                                'ja_JP.eucJP',
    'ja_jp.euc':                            'ja_JP.eucJP',
    'ja_jp.mscode':                         'ja_JP.SJIS',
    'ja_jp.pck':                            'ja_JP.SJIS',
    'japan':                                'ja_JP.eucJP',
    'japanese':                             'ja_JP.eucJP',
    'japanese-euc':                         'ja_JP.eucJP',
    'japanese.euc':                         'ja_JP.eucJP',
    'jp_jp':                                'ja_JP.eucJP',
    'ka':                                   'ka_GE.GEORGIAN-ACADEMY',
    'ka_ge':                                'ka_GE.GEORGIAN-ACADEMY',
    'ka_ge.georgianacademy':                'ka_GE.GEORGIAN-ACADEMY',
    'ka_ge.georgianps':                     'ka_GE.GEORGIAN-PS',
    'ka_ge.georgianrs':                     'ka_GE.GEORGIAN-ACADEMY',
    'kab_dz':                               'kab_DZ.UTF-8',
    'kk_kz':                                'kk_KZ.ptcp154',
    'kl':                                   'kl_GL.ISO8859-1',
    'kl_gl':                                'kl_GL.ISO8859-1',
    'km_kh':                                'km_KH.UTF-8',
    'kn':                                   'kn_IN.UTF-8',
    'kn_in':                                'kn_IN.UTF-8',
    'ko':                                   'ko_KR.eucKR',
    'ko_kr':                                'ko_KR.eucKR',
    'ko_kr.euc':                            'ko_KR.eucKR',
    'kok_in':                               'kok_IN.UTF-8',
    'korean':                               'ko_KR.eucKR',
    'korean.euc':                           'ko_KR.eucKR',
    'ks':                                   'ks_IN.UTF-8',
    'ks_in':                                'ks_IN.UTF-8',
    'ks_in@devanagari.utf8':                'ks_IN.UTF-8@devanagari',
    'ku_tr':                                'ku_TR.ISO8859-9',
    'kv_ru':                                'kv_RU.UTF-8',
    'kw':                                   'kw_GB.ISO8859-1',
    'kw_gb':                                'kw_GB.ISO8859-1',
    'ky':                                   'ky_KG.UTF-8',
    'ky_kg':                                'ky_KG.UTF-8',
    'lb_lu':                                'lb_LU.UTF-8',
    'lg_ug':                                'lg_UG.ISO8859-10',
    'li_be':                                'li_BE.UTF-8',
    'li_nl':                                'li_NL.UTF-8',
    'lij_it':                               'lij_IT.UTF-8',
    'lithuanian':                           'lt_LT.ISO8859-13',
    'ln_cd':                                'ln_CD.UTF-8',
    'lo':                                   'lo_LA.MULELAO-1',
    'lo_la':                                'lo_LA.MULELAO-1',
    'lo_la.cp1133':                         'lo_LA.IBM-CP1133',
    'lo_la.ibmcp1133':                      'lo_LA.IBM-CP1133',
    'lo_la.mulelao1':                       'lo_LA.MULELAO-1',
    'lt':                                   'lt_LT.ISO8859-13',
    'lt_lt':                                'lt_LT.ISO8859-13',
    'ltg_lv.utf8':                          'ltg_LV.UTF-8',
    'lv':                                   'lv_LV.ISO8859-13',
    'lv_lv':                                'lv_LV.ISO8859-13',
    'lzh_tw':                               'lzh_TW.UTF-8',
    'mag_in':                               'mag_IN.UTF-8',
    'mai':                                  'mai_IN.UTF-8',
    'mai_in':                               'mai_IN.UTF-8',
    'mai_np':                               'mai_NP.UTF-8',
    'mdf_ru':                               'mdf_RU.UTF-8',
    'mfe_mu':                               'mfe_MU.UTF-8',
    'mg_mg':                                'mg_MG.ISO8859-15',
    'mhr_ru':                               'mhr_RU.UTF-8',
    'mi':                                   'mi_NZ.ISO8859-1',
    'mi_nz':                                'mi_NZ.ISO8859-1',
    'miq_ni':                               'miq_NI.UTF-8',
    'mjw_in':                               'mjw_IN.UTF-8',
    'mk':                                   'mk_MK.ISO8859-5',
    'mk_mk':                                'mk_MK.ISO8859-5',
    'ml':                                   'ml_IN.UTF-8',
    'ml_in':                                'ml_IN.UTF-8',
    'mn_mn':                                'mn_MN.UTF-8',
    'mni_in':                               'mni_IN.UTF-8',
    'mnw_mm':                               'mnw_MM.UTF-8',
    'mr':                                   'mr_IN.UTF-8',
    'mr_in':                                'mr_IN.UTF-8',
    'ms':                                   'ms_MY.ISO8859-1',
    'ms_my':                                'ms_MY.ISO8859-1',
    'mt':                                   'mt_MT.ISO8859-3',
    'mt_mt':                                'mt_MT.ISO8859-3',
    'my_mm':                                'my_MM.UTF-8',
    'nan_tw':                               'nan_TW.UTF-8',
    'nb':                                   'nb_NO.ISO8859-1',
    'nb_no':                                'nb_NO.ISO8859-1',
    'nds_de':                               'nds_DE.UTF-8',
    'nds_nl':                               'nds_NL.UTF-8',
    'ne_np':                                'ne_NP.UTF-8',
    'nhn_mx':                               'nhn_MX.UTF-8',
    'niu_nu':                               'niu_NU.UTF-8',
    'niu_nz':                               'niu_NZ.UTF-8',
    'nl':                                   'nl_NL.ISO8859-1',
    'nl_aw':                                'nl_AW.UTF-8',
    'nl_be':                                'nl_BE.ISO8859-1',
    'nl_nl':                                'nl_NL.ISO8859-1',
    'nn':                                   'nn_NO.ISO8859-1',
    'nn_no':                                'nn_NO.ISO8859-1',
    'no':                                   'no_NO.ISO8859-1',
    'no@nynorsk':                           'ny_NO.ISO8859-1',
    'no_no':                                'no_NO.ISO8859-1',
    'no_no.iso88591@bokmal':                'no_NO.ISO8859-1',
    'no_no.iso88591@nynorsk':               'no_NO.ISO8859-1',
    'norwegian':                            'no_NO.ISO8859-1',
    'nr':                                   'nr_ZA.ISO8859-1',
    'nr_za':                                'nr_ZA.ISO8859-1',
    'nso':                                  'nso_ZA.ISO8859-15',
    'nso_za':                               'nso_ZA.ISO8859-15',
    'ny':                                   'ny_NO.ISO8859-1',
    'ny_no':                                'ny_NO.ISO8859-1',
    'nynorsk':                              'nn_NO.ISO8859-1',
    'oc':                                   'oc_FR.ISO8859-1',
    'oc_fr':                                'oc_FR.ISO8859-1',
    'om_et':                                'om_ET.UTF-8',
    'om_ke':                                'om_KE.ISO8859-1',
    'or':                                   'or_IN.UTF-8',
    'or_in':                                'or_IN.UTF-8',
    'os_ru':                                'os_RU.UTF-8',
    'pa':                                   'pa_IN.UTF-8',
    'pa_in':                                'pa_IN.UTF-8',
    'pa_pk':                                'pa_PK.UTF-8',
    'pap_an':                               'pap_AN.UTF-8',
    'pap_aw':                               'pap_AW.UTF-8',
    'pap_cw':                               'pap_CW.UTF-8',
    'pd':                                   'pd_US.ISO8859-1',
    'pd_de':                                'pd_DE.ISO8859-1',
    'pd_us':                                'pd_US.ISO8859-1',
    'ph':                                   'ph_PH.ISO8859-1',
    'ph_ph':                                'ph_PH.ISO8859-1',
    'pl':                                   'pl_PL.ISO8859-2',
    'pl_pl':                                'pl_PL.ISO8859-2',
    'polish':                               'pl_PL.ISO8859-2',
    'portuguese':                           'pt_PT.ISO8859-1',
    'portuguese_brazil':                    'pt_BR.ISO8859-1',
    'posix':                                'C',
    'posix-utf2':                           'C',
    'pp':                                   'pp_AN.ISO8859-1',
    'pp_an':                                'pp_AN.ISO8859-1',
    'ps_af':                                'ps_AF.UTF-8',
    'pt':                                   'pt_PT.ISO8859-1',
    'pt_br':                                'pt_BR.ISO8859-1',
    'pt_pt':                                'pt_PT.ISO8859-1',
    'quz_pe':                               'quz_PE.UTF-8',
    'raj_in':                               'raj_IN.UTF-8',
    'rif_ma':                               'rif_MA.UTF-8',
    'ro':                                   'ro_RO.ISO8859-2',
    'ro_ro':                                'ro_RO.ISO8859-2',
    'romanian':                             'ro_RO.ISO8859-2',
    'ru':                                   'ru_RU.UTF-8',
    'ru_ru':                                'ru_RU.UTF-8',
    'ru_ua':                                'ru_UA.KOI8-U',
    'rumanian':                             'ro_RO.ISO8859-2',
    'russian':                              'ru_RU.ISO8859-5',
    'rw':                                   'rw_RW.ISO8859-1',
    'rw_rw':                                'rw_RW.ISO8859-1',
    'sa_in':                                'sa_IN.UTF-8',
    'sah_ru':                               'sah_RU.UTF-8',
    'sat_in':                               'sat_IN.UTF-8',
    'sc_it':                                'sc_IT.UTF-8',
    'scn_it':                               'scn_IT.UTF-8',
    'sd':                                   'sd_IN.UTF-8',
    'sd_in':                                'sd_IN.UTF-8',
    'sd_in@devanagari.utf8':                'sd_IN.UTF-8@devanagari',
    'sd_pk':                                'sd_PK.UTF-8',
    'se_no':                                'se_NO.UTF-8',
    'serbocroatian':                        'sr_RS.UTF-8@latin',
    'sgs_lt':                               'sgs_LT.UTF-8',
    'sh':                                   'sr_RS.UTF-8@latin',
    'sh_ba.iso88592@bosnia':                'sr_CS.ISO8859-2',
    'sh_hr':                                'sh_HR.ISO8859-2',
    'sh_hr.iso88592':                       'hr_HR.ISO8859-2',
    'sh_sp':                                'sr_CS.ISO8859-2',
    'sh_yu':                                'sr_RS.UTF-8@latin',
    'shn_mm':                               'shn_MM.UTF-8',
    'shs_ca':                               'shs_CA.UTF-8',
    'si':                                   'si_LK.UTF-8',
    'si_lk':                                'si_LK.UTF-8',
    'sid_et':                               'sid_ET.UTF-8',
    'sinhala':                              'si_LK.UTF-8',
    'sk':                                   'sk_SK.ISO8859-2',
    'sk_sk':                                'sk_SK.ISO8859-2',
    'sl':                                   'sl_SI.ISO8859-2',
    'sl_cs':                                'sl_CS.ISO8859-2',
    'sl_si':                                'sl_SI.ISO8859-2',
    'slovak':                               'sk_SK.ISO8859-2',
    'slovene':                              'sl_SI.ISO8859-2',
    'slovenian':                            'sl_SI.ISO8859-2',
    'sm_ws':                                'sm_WS.UTF-8',
    'so_dj':                                'so_DJ.ISO8859-1',
    'so_et':                                'so_ET.UTF-8',
    'so_ke':                                'so_KE.ISO8859-1',
    'so_so':                                'so_SO.ISO8859-1',
    'sp':                                   'sr_CS.ISO8859-5',
    'sp_yu':                                'sr_CS.ISO8859-5',
    'spanish':                              'es_ES.ISO8859-1',
    'spanish_spain':                        'es_ES.ISO8859-1',
    'sq':                                   'sq_AL.ISO8859-2',
    'sq_al':                                'sq_AL.ISO8859-2',
    'sq_mk':                                'sq_MK.UTF-8',
    'sr':                                   'sr_RS.UTF-8',
    'sr@cyrillic':                          'sr_RS.UTF-8',
    'sr@latn':                              'sr_RS.UTF-8@latin',
    'sr_cs':                                'sr_CS.UTF-8',
    'sr_cs.iso88592@latn':                  'sr_CS.ISO8859-2',
    'sr_cs@latn':                           'sr_CS.UTF-8@latin',
    'sr_me':                                'sr_ME.UTF-8',
    'sr_rs':                                'sr_RS.UTF-8',
    'sr_rs@latn':                           'sr_RS.UTF-8@latin',
    'sr_sp':                                'sr_CS.ISO8859-2',
    'sr_yu':                                'sr_RS.UTF-8@latin',
    'sr_yu.cp1251@cyrillic':                'sr_CS.CP1251',
    'sr_yu.iso88592':                       'sr_CS.ISO8859-2',
    'sr_yu.iso88595':                       'sr_CS.ISO8859-5',
    'sr_yu.iso88595@cyrillic':              'sr_CS.ISO8859-5',
    'sr_yu.microsoftcp1251@cyrillic':       'sr_CS.CP1251',
    'sr_yu.utf8':                           'sr_RS.UTF-8',
    'sr_yu.utf8@cyrillic':                  'sr_RS.UTF-8',
    'sr_yu@cyrillic':                       'sr_RS.UTF-8',
    'ss':                                   'ss_ZA.ISO8859-1',
    'ss_za':                                'ss_ZA.ISO8859-1',
    'ssy_er':                               'ssy_ER.UTF-8',
    'st':                                   'st_ZA.ISO8859-1',
    'st_za':                                'st_ZA.ISO8859-1',
    'su_id':                                'su_ID.UTF-8',
    'sv':                                   'sv_SE.ISO8859-1',
    'sv_fi':                                'sv_FI.ISO8859-1',
    'sv_se':                                'sv_SE.ISO8859-1',
    'sw_ke':                                'sw_KE.UTF-8',
    'sw_tz':                                'sw_TZ.UTF-8',
    'swedish':                              'sv_SE.ISO8859-1',
    'syr':                                  'syr.UTF-8',
    'szl_pl':                               'szl_PL.UTF-8',
    'ta':                                   'ta_IN.TSCII-0',
    'ta_in':                                'ta_IN.TSCII-0',
    'ta_in.tscii':                          'ta_IN.TSCII-0',
    'ta_in.tscii0':                         'ta_IN.TSCII-0',
    'ta_lk':                                'ta_LK.UTF-8',
    'tcy_in.utf8':                          'tcy_IN.UTF-8',
    'te':                                   'te_IN.UTF-8',
    'te_in':                                'te_IN.UTF-8',
    'tg':                                   'tg_TJ.KOI8-C',
    'tg_tj':                                'tg_TJ.KOI8-C',
    'th':                                   'th_TH.ISO8859-11',
    'th_th':                                'th_TH.ISO8859-11',
    'th_th.tactis':                         'th_TH.TIS620',
    'th_th.tis620':                         'th_TH.TIS620',
    'thai':                                 'th_TH.ISO8859-11',
    'the_np':                               'the_NP.UTF-8',
    'ti_er':                                'ti_ER.UTF-8',
    'ti_et':                                'ti_ET.UTF-8',
    'tig_er':                               'tig_ER.UTF-8',
    'tk_tm':                                'tk_TM.UTF-8',
    'tl':                                   'tl_PH.ISO8859-1',
    'tl_ph':                                'tl_PH.ISO8859-1',
    'tn':                                   'tn_ZA.ISO8859-15',
    'tn_za':                                'tn_ZA.ISO8859-15',
    'to_to':                                'to_TO.UTF-8',
    'tok':                                  'tok.UTF-8',
    'tpi_pg':                               'tpi_PG.UTF-8',
    'tr':                                   'tr_TR.ISO8859-9',
    'tr_cy':                                'tr_CY.ISO8859-9',
    'tr_tr':                                'tr_TR.ISO8859-9',
    'ts':                                   'ts_ZA.ISO8859-1',
    'ts_za':                                'ts_ZA.ISO8859-1',
    'tt':                                   'tt_RU.TATAR-CYR',
    'tt_ru':                                'tt_RU.TATAR-CYR',
    'tt_ru.tatarcyr':                       'tt_RU.TATAR-CYR',
    'tt_ru@iqtelif':                        'tt_RU.UTF-8@iqtelif',
    'turkish':                              'tr_TR.ISO8859-9',
    'ug_cn':                                'ug_CN.UTF-8',
    'uk':                                   'uk_UA.KOI8-U',
    'uk_ua':                                'uk_UA.KOI8-U',
    'univ.utf8':                            'en_US.UTF-8',
    'universal.utf8@ucs4':                  'en_US.UTF-8',
    'unm_us':                               'unm_US.UTF-8',
    'ur':                                   'ur_PK.CP1256',
    'ur_in':                                'ur_IN.UTF-8',
    'ur_pk':                                'ur_PK.CP1256',
    'uz':                                   'uz_UZ.UTF-8',
    'uz_uz':                                'uz_UZ.UTF-8',
    'uz_uz@cyrillic':                       'uz_UZ.UTF-8',
    've':                                   've_ZA.UTF-8',
    've_za':                                've_ZA.UTF-8',
    'vi':                                   'vi_VN.TCVN',
    'vi_vn':                                'vi_VN.TCVN',
    'vi_vn.tcvn':                           'vi_VN.TCVN',
    'vi_vn.tcvn5712':                       'vi_VN.TCVN',
    'vi_vn.viscii':                         'vi_VN.VISCII',
    'vi_vn.viscii111':                      'vi_VN.VISCII',
    'wa':                                   'wa_BE.ISO8859-1',
    'wa_be':                                'wa_BE.ISO8859-1',
    'wae_ch':                               'wae_CH.UTF-8',
    'wal_et':                               'wal_ET.UTF-8',
    'wo_sn':                                'wo_SN.UTF-8',
    'xh':                                   'xh_ZA.ISO8859-1',
    'xh_za':                                'xh_ZA.ISO8859-1',
    'yi':                                   'yi_US.CP1255',
    'yi_us':                                'yi_US.CP1255',
    'yo_ng':                                'yo_NG.UTF-8',
    'yue_hk':                               'yue_HK.UTF-8',
    'yuw_pg':                               'yuw_PG.UTF-8',
    'zgh_ma':                               'zgh_MA.UTF-8',
    'zh':                                   'zh_CN.eucCN',
    'zh_cn':                                'zh_CN.gb2312',
    'zh_cn.big5':                           'zh_TW.big5',
    'zh_cn.euc':                            'zh_CN.eucCN',
    'zh_hk':                                'zh_HK.big5hkscs',
    'zh_hk.big5hk':                         'zh_HK.big5hkscs',
    'zh_sg':                                'zh_SG.GB2312',
    'zh_sg.gbk':                            'zh_SG.GBK',
    'zh_tw':                                'zh_TW.big5',
    'zh_tw.euc':                            'zh_TW.eucTW',
    'zh_tw.euctw':                          'zh_TW.eucTW',
    'zu':                                   'zu_ZA.ISO8859-1',
    'zu_za':                                'zu_ZA.ISO8859-1',
}


def _replace_encoding(code, encoding):
    if '.' in code:
        langname = code[:code.index('.')]
    else:
        langname = code
    # Convert the encoding to a C lib compatible encoding string
    norm_encoding = normalize_encoding(encoding)
    #print('norm encoding: %r' % norm_encoding)
    norm_encoding = _encoding_aliases.get(norm_encoding.lower(), norm_encoding)
    #print('aliased encoding: %r' % norm_encoding)
    encoding = norm_encoding
    norm_encoding = norm_encoding.lower()
    if norm_encoding in locale_encoding_alias:
        encoding = locale_encoding_alias[norm_encoding]
    else:
        norm_encoding = norm_encoding.replace('_', '')
        norm_encoding = norm_encoding.replace('-', '')
        if norm_encoding in locale_encoding_alias:
            encoding = locale_encoding_alias[norm_encoding]
    #print('found encoding %r' % encoding)
    return langname + '.' + encoding

def _append_modifier(code, modifier):
    if modifier == 'euro':
        if '.' not in code:
            return code + '.ISO8859-15'
        _, _, encoding = code.partition('.')
        if encoding in ('ISO8859-15', 'UTF-8'):
            return code
        if encoding == 'ISO8859-1':
            return _replace_encoding(code, 'ISO8859-15')
    return code + '@' + modifier

def normalize(localename):

    """ Returns a normalized locale code for the given locale
        name.

        The returned locale code is formatted for use with
        setlocale().

        If normalization fails, the original name is returned
        unchanged.

        If the given encoding is not known, the function defaults to
        the default encoding for the locale code just like setlocale()
        does.

    """
    # Normalize the locale name and extract the encoding and modifier
    code = localename.lower()
    if ':' in code:
        # ':' is sometimes used as encoding delimiter.
        code = code.replace(':', '.')
    if '@' in code:
        code, modifier = code.split('@', 1)
    else:
        modifier = ''
    if '.' in code:
        langname, encoding = code.split('.')[:2]
    else:
        langname = code
        encoding = ''

    # First lookup: fullname (possibly with encoding and modifier)
    lang_enc = langname
    if encoding:
        norm_encoding = encoding.replace('-', '')
        norm_encoding = norm_encoding.replace('_', '')
        lang_enc += '.' + norm_encoding
    lookup_name = lang_enc
    if modifier:
        lookup_name += '@' + modifier
    code = locale_alias.get(lookup_name, None)
    if code is not None:
        return code
    #print('first lookup failed')

    if modifier:
        # Second try: fullname without modifier (possibly with encoding)
        code = locale_alias.get(lang_enc, None)
        if code is not None:
            #print('lookup without modifier succeeded')
            if '@' not in code:
                return _append_modifier(code, modifier)
            if code.split('@', 1)[1].lower() == modifier:
                return code
        #print('second lookup failed')

    if encoding:
        # Third try: langname (without encoding, possibly with modifier)
        lookup_name = langname
        if modifier:
            lookup_name += '@' + modifier
        code = locale_alias.get(lookup_name, None)
        if code is not None:
            #print('lookup without encoding succeeded')
            if '@' not in code:
                return _replace_encoding(code, encoding)
            code, modifier = code.split('@', 1)
            return _replace_encoding(code, encoding) + '@' + modifier

        if modifier:
            # Fourth try: langname (without encoding and modifier)
            code = locale_alias.get(langname, None)
            if code is not None:
                #print('lookup without modifier and encoding succeeded')
                if '@' not in code:
                    code = _replace_encoding(code, encoding)
                    return _append_modifier(code, modifier)
                code, defmod = code.split('@', 1)
                if defmod.lower() == modifier:
                    return _replace_encoding(code, encoding) + '@' + defmod

    return localename

